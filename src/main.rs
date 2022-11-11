use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
};

use lazy_static::lazy_static;
use regex::Regex;
use scraper::{ElementRef, Html, Selector};

const CHAINSAWMAN_WIKI_BASE_URL: &'static str = "https://chainsaw-man.fandom.com";

#[derive(Debug)]
struct Devil {
    pub devil_name: String,
    pub alias_name: Option<String>,
    pub wiki_url: String,
    pub category: String,
}

#[derive(Debug)]
struct DevilDetail {
    pub names: HashMap<&'static str, DevilName>,
    pub image_src: Option<String>,
    pub gender: Option<String>,
    pub birthplace: Option<String>,
    pub status: Option<String>,
    pub occupations: Vec<String>,
    pub affiliations: Vec<String>,
    pub contracts: Vec<String>,
    pub relatives: Vec<String>,
    pub abilities: HashMap<&'static str, Vec<Ability>>,
}

/**
* Names will be stored as a hashmap, where the key is the language code
* */
#[derive(Debug)]
struct DevilName {
    pub devil_name: String,
    pub alias_name: Option<String>,
}

#[derive(Debug)]
struct Ability {
    pub name: String,
    pub description: String,
    pub abilities: Vec<Ability>,
}

// TODO: Improve error handling to not rely on std::io::Error
async fn scrape_devils() -> Result<Vec<Devil>, Error> {
    // key: Category, value: selector for the devil category div
    let map = HashMap::<&'static str, &'static str>::from([
        ("Normal Devils", r#"div[id="gallery-0"]"#),
        ("Primal Devils", r#"div[id="gallery-1"]"#),
        ("Reincarnated Devils", r#"div[id="gallery-2"]"#),
        ("Fiends", r#"div[id="gallery-4"]"#),
        ("Hybrids", r#"div[id="gallery-5"]"#),
    ]);

    let mut devils: Vec<Devil> = Vec::new();

    let devils_page = format!("{}/wiki/devil", CHAINSAWMAN_WIKI_BASE_URL);
    let response = match reqwest::get(devils_page).await {
        Ok(html) => html,
        Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
    };

    let html = match response.text().await {
        Ok(text) => text,
        Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
    };

    let document = Html::parse_document(&html);
    for (key, value) in map.into_iter() {
        let root_selector = Selector::parse(value).unwrap();
        let children_selector = Selector::parse(r#"div[class="wikia-gallery-item"]"#).unwrap();

        let root = document.select(&root_selector).next().unwrap();
        for el in root.select(&children_selector) {
            let link_selector = match Selector::parse(r#"div[class="lightbox-caption"] > a"#) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let a = match el.select(&link_selector).next() {
                Some(el) => el,
                None => continue,
            };
            let href = match a.value().attr("href") {
                Some(href) => href,
                None => continue,
            };

            let wiki_url = format!("{}{}", CHAINSAWMAN_WIKI_BASE_URL, href);

            let raw_name = a.text().collect::<String>();
            let splitted_names = raw_name.split("/");
            let names = splitted_names.collect::<Vec<&str>>();

            let mut devil_name: String = names[0].into();
            let mut alias_name: Option<String> = None;

            if names.len() == 2 {
                alias_name = Some(names[0].into());
                devil_name = names[1].into();
            }

            devils.push(Devil {
                devil_name,
                alias_name,
                wiki_url,
                category: key.into(),
            });
        }
    }

    Ok(devils)
}

const SECTION_NAME: &'static str = "Name";
const SECTION_BIOLOGICAL: &'static str = "Biological Information";
const SECTION_PROFESSIONAL: &'static str = "Professional Information";

fn scrape_abilities(document: &Html, selector: Selector) -> Vec<Ability> {
    let mut abilities = Vec::new();

    if let Some(el) = document.select(&selector).next() {
        let parent: ElementRef = el.parent().and_then(ElementRef::wrap).unwrap();
        for sibling in parent.next_siblings() {
            if let Some(el) = sibling.value().as_element() {
                if el.name() != "ul" && el.name() != "figure" {
                    break;
                }

                let ul = match ElementRef::wrap(sibling) {
                    Some(ul) => ul,
                    None => continue,
                };

                let li_selector = Selector::parse(r#":scope > li"#).unwrap();
                for li in ul.select(&li_selector) {
                    get_abilities(li, &mut abilities);
                }
            }
        }
    }

    abilities
}

fn get_abilities(element: ElementRef, abilities: &mut Vec<Ability>) {
    lazy_static! {
        static ref REF_CLEANER: Regex = Regex::new(r#"\[\d+\]/gm"#).unwrap();
    }

    let texts = element.text().collect::<String>();
    let texts = texts
        .split(":")
        .into_iter()
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    if texts.len() < 2 {
        return;
    }

    let name = texts[0].trim().to_string();
    let description = REF_CLEANER.replace_all(&texts[1].trim().to_string(), "").to_string(); 

    let mut child_abilities: Vec<Ability> = Vec::new();
    let child_selector = Selector::parse(r#"ul > li"#).unwrap();

    for child in element.select(&child_selector) {
        get_abilities(child, &mut child_abilities);
    }

    abilities.push(Ability {
        name,
        description,
        abilities: child_abilities,
    });
}

fn print_ability_tree(level: i32, ability: &Ability, with_description: bool) {
    println!("level: {} - ability: {}", level, ability.name);
    if with_description {
        println!("description: {}", ability.description);
    }

    for a in &ability.abilities {
        print_ability_tree(level + 1, &a, with_description);
    }
}

async fn scrape_devil_detail(url: String) -> Result<DevilDetail, Error> {
    lazy_static! {
        // <a> element
        static ref TEXT_MATCHER_A: Regex = Regex::new(r#"<a[^>]*>(.*?)</a>"#).unwrap();
    }

    let response = match reqwest::get(url).await {
        Ok(html) => html,
        Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
    };
    let html = match response.text().await {
        Ok(text) => text,
        Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
    };
    let document = Html::parse_document(&html);

    let mut names: HashMap<&'static str, DevilName> = HashMap::new();

    let mut gender: Option<String> = None;
    let mut birthplace: Option<String> = None;
    let mut status: Option<String> = None;

    let mut occupations: Vec<String> = Vec::new();
    let mut affiliations: Vec<String> = Vec::new();
    let mut contracts: Vec<String> = Vec::new();
    let mut relatives: Vec<String> = Vec::new();

    let mut abilities: HashMap<&'static str, Vec<Ability>> = HashMap::new();

    // TODO: parse image
    let abilities_selector: HashMap<&'static str, &'static str> = HashMap::from([
        ("physical", r#"span[id="Physical_Abilities"]"#),
        ("physical_prowess", r#"span[id="Physical_Prowess"]"#),
        ("devil", r#"span[id="Devil_Powers"]"#),
        ("supernatural", r#"span[id="Supernatural_Abilities"]"#),
    ]);
    for (key, value) in abilities_selector {
        let current_abilities = scrape_abilities(&document, Selector::parse(value).unwrap());
        for ab in &current_abilities {
            print_ability_tree(0, ab, true);
        }
        if current_abilities.len() > 0 {
            abilities.insert(key, current_abilities);
        }
    }

    let section_selector =
        Selector::parse(r#"section[class="pi-item pi-group pi-border-color"]"#).unwrap();
    for el in document.select(&section_selector) {
        let h2_selector = Selector::parse("h2").unwrap();

        let h2 = match el.select(&h2_selector).next() {
            Some(h2) => h2,
            None => continue,
        };

        let section_name = h2.text().collect::<String>();
        if section_name == SECTION_NAME {
            {
                let kanji_selector = Selector::parse(r#"div[data-source="kanji"] > div"#).unwrap();
                let div = match el.select(&kanji_selector).next() {
                    Some(div) => div,
                    None => continue,
                };

                let kanji = div.text().collect::<Vec<_>>();

                let devil_name = kanji[0].to_string();

                // TODO: handle how to clean the kanjis in the top of the alias name (?)
                let alias_name: Option<String> = None;
                // if kanji.len() > 1 {
                //     alias_name = Some(kanji[1..].join(""));
                // }

                names.insert(
                    "kanji",
                    DevilName {
                        devil_name,
                        alias_name,
                    },
                );
            }
            {
                let mut romajis: Vec<String> = Vec::new();

                let romaji_selector =
                    Selector::parse(r#"div[data-source="romaji"] > div > div > i"#).unwrap();
                for romaji in el.select(&romaji_selector) {
                    romajis.push(romaji.text().collect::<String>());
                }

                let devil_name = romajis[0].to_string();
                let alias_name: Option<String> = if romajis.len() > 1 {
                    Some(romajis[1].to_string())
                } else {
                    None
                };

                names.insert(
                    "romaji",
                    DevilName {
                        devil_name,
                        alias_name,
                    },
                );
            }
        } else if section_name == SECTION_BIOLOGICAL {
            let gender_selector = Selector::parse(r#"div[data-source="gender"] > div"#).unwrap();
            let birthplace_selector =
                Selector::parse(r#"div[data-source="birthplace"] > div"#).unwrap();
            let status_selector =
                Selector::parse(r#"div[data-source="status"] > div > div"#).unwrap();

            if let Some(div) = el.select(&gender_selector).next() {
                gender = Some(div.text().collect::<String>());
            }
            if let Some(div) = el.select(&birthplace_selector).next() {
                birthplace = Some(div.text().collect::<String>());
            }
            if let Some(div) = el.select(&status_selector).next() {
                status = Some(div.inner_html().trim().to_string());
            }
        } else if section_name == SECTION_PROFESSIONAL {
            let occupation_selector =
                Selector::parse(r#"div[data-source="occupation"] > div"#).unwrap();
            let affiliation_selector =
                Selector::parse(r#"div[data-source="affiliation"] > div > ul > li"#).unwrap();
            let contract_selector =
                Selector::parse(r#"div[data-source="contracted humans"] > div"#).unwrap();
            let relative_selector =
                Selector::parse(r#"div[data-source="relatives"] > div"#).unwrap();

            if let Some(div) = el.select(&occupation_selector).next() {
                let texts = div.text().collect::<Vec<_>>();
                for t in texts {
                    occupations.push(t.trim().to_string());
                }
            }

            for li in el.select(&affiliation_selector) {
                let mut curr: Vec<String> = Vec::new();

                let nested_li_selector = Selector::parse(r#"ul > li"#).unwrap();
                for nested_li in li.select(&nested_li_selector) {
                    let text = nested_li.text().collect::<String>();
                    curr.push(text);
                }

                if curr.len() == 0 {
                    let text = li.text().collect::<String>();
                    curr.push(text);
                }

                affiliations.append(&mut curr);
            }

            if let Some(div) = el.select(&contract_selector).next() {
                let texts = div.text().collect::<Vec<_>>();
                for t in texts {
                    contracts.push(t.trim().to_string());
                }
            }

            if let Some(div) = el.select(&relative_selector).next() {
                let html = div.inner_html();
                let texts = html.split("<br>").into_iter().collect::<Vec<&str>>();
                for text in texts {
                    let result: Option<String> = if TEXT_MATCHER_A.is_match(text) {
                        if let Some(groups) = TEXT_MATCHER_A.captures(text) {
                            Some(groups[1].to_string())
                        } else {
                            None
                        }
                    } else {
                        Some(text.trim().to_string())
                    };

                    if let Some(result) = result {
                        relatives.push(result);
                    }
                }
            }
        }
    }

    Ok(DevilDetail {
        names,
        image_src: None,
        gender,
        birthplace,
        status,
        occupations,
        affiliations,
        contracts,
        relatives,
        abilities,
    })
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // let _devils = match scrape_devils().await {
    //     Ok(devils) => devils,
    //     Err(e) => panic!("{}", e.to_string())
    // };

    if let Err(e) =
        scrape_devil_detail("https://chainsaw-man.fandom.com/wiki/Makima".into()).await
    {
        panic!("{}", e.to_string());
    }
}
