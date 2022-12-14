use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use duplicate::duplicate_item;
use lazy_static::lazy_static;
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use tokio::{
    sync::{Mutex, Semaphore},
    task::JoinHandle,
};

use crate::{
    models::devil::{Ability, Devil, DevilDetail, DevilName},
    services::DevilDataSource,
};

const CHAINSAWMAN_WIKI_BASE_URL: &'static str = "https://chainsaw-man.fandom.com";

const SECTION_NAME: &'static str = "Name";
const SECTION_BIOLOGICAL: &'static str = "Biological Information";
const SECTION_PROFESSIONAL: &'static str = "Professional Information";

const NUM_OF_SCRAPER_WORKERS: usize = 5;
const TASK_FINISH_DELAY_MS: u64 = 3000;

pub struct DevilScraper {}

impl DevilScraper {
    pub fn new() -> Self {
        DevilScraper {}
    }
}

#[duplicate_item(Interface; [DevilDataSource])]
#[async_trait]
impl Interface for DevilScraper {
    async fn scrape(&self) -> Result<Vec<DevilDetail>, std::io::Error> {
        let devils = match scrape_devils().await {
            Ok(devils) => devils,
            Err(e) => panic!("{}", e.to_string()),
        };

        let result: Arc<Mutex<Vec<DevilDetail>>> = Arc::new(Mutex::new(Vec::new()));

        let semaphore = Arc::new(Semaphore::new(NUM_OF_SCRAPER_WORKERS));
        let mut join_handles: Vec<JoinHandle<()>> = Vec::new();

        for devil in devils {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let result = result.clone();

            join_handles.push(tokio::spawn(async move {
                let detail = match scrape_devil_detail(devil.wiki_url.clone()).await {
                    Ok(detail) => detail,
                    Err(_) => return (),
                };

                let mut guard = result.lock().await;
                guard.push(detail);
                std::mem::drop(guard);

                tokio::time::sleep(Duration::from_millis(TASK_FINISH_DELAY_MS)).await;
                drop(permit);
            }));
        }

        for handle in join_handles {
            handle.await.unwrap();
        }

        let result = result.lock().await;
        Ok(result.to_vec())
    }
}

#[allow(dead_code)]
fn print_ability_tree(level: i32, ability: &Ability, with_description: bool) {
    println!("level: {} - ability: {}", level, ability.name);
    if with_description {
        println!("description: {}", ability.description);
    }

    for a in &ability.abilities {
        print_ability_tree(level + 1, &a, with_description);
    }
}

// TODO: Improve error handling to not rely on std::io::Error
async fn scrape_devils() -> Result<Vec<Devil>, Error> {
    // key: Category, value: selector for the devil category div
    let map = HashMap::<&'static str, Selector>::from([
        (
            "Normal Devils",
            Selector::parse(r#"div[id="gallery-0"]"#).unwrap(),
        ),
        (
            "Primal Devils",
            Selector::parse(r#"div[id="gallery-1"]"#).unwrap(),
        ),
        (
            "Reincarnated Devils",
            Selector::parse(r#"div[id="gallery-2"]"#).unwrap(),
        ),
        ("Fiends", Selector::parse(r#"div[id="gallery-4"]"#).unwrap()),
        (
            "Hybrids",
            Selector::parse(r#"div[id="gallery-5"]"#).unwrap(),
        ),
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
    for (category, root_selector) in map.into_iter() {
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
                category: category.to_string(),
            });
        }
    }

    Ok(devils)
}

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
    let description = REF_CLEANER
        .replace_all(&texts[1].trim().to_string(), "")
        .to_string();

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

    let mut names: HashMap<String, DevilName> = HashMap::new();

    let mut gender: Option<String> = None;
    let mut birthplace: Option<String> = None;
    let mut status: Option<String> = None;

    let mut occupations: Vec<String> = Vec::new();
    let mut affiliations: Vec<String> = Vec::new();
    let mut contracts: Vec<String> = Vec::new();
    let mut relatives: Vec<String> = Vec::new();

    let mut abilities: HashMap<String, Vec<Ability>> = HashMap::new();

    // TODO: parse image
    let abilities_selector: HashMap<&'static str, Selector> = HashMap::from([
        (
            "physical",
            Selector::parse(r#"span[id="Physical_Abilities"]"#).unwrap(),
        ),
        (
            "physical_prowess",
            Selector::parse(r#"span[id="Physical_Prowess"]"#).unwrap(),
        ),
        (
            "devil",
            Selector::parse(r#"span[id="Devil_Powers"]"#).unwrap(),
        ),
        (
            "supernatural",
            Selector::parse(r#"span[id="Supernatural_Abilities"]"#).unwrap(),
        ),
    ]);
    for (ability_type, selector) in abilities_selector {
        let current_abilities = scrape_abilities(&document, selector);
        if current_abilities.len() > 0 {
            abilities.insert(ability_type.to_string(), current_abilities);
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
                    "kanji".to_string(),
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
                    "romaji".to_string(),
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
