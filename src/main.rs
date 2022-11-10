use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
};

use scraper::{Html, Selector};

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
    pub age: Option<i32>,
    pub status: Option<String>,
    pub occupations: Vec<String>,
    pub affiliations: Vec<String>,
    pub contracts: Vec<String>,
    pub relatives: Vec<String>,
}

/**
* Names will be stored as a hashmap, where the key is the language code
* */
#[derive(Debug)]
struct DevilName {
    pub devil_name: String,
    pub alias_name: Option<String>,
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

async fn scrape_devil_detail(url: String) -> Result<DevilDetail, Error> {
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
    let mut age: Option<i32> = None;

    let mut occupations: Vec<String> = Vec::new();
    let mut affiliations: Vec<String> = Vec::new();
    let mut contracts: Vec<String> = Vec::new();
    let mut relatives: Vec<String> = Vec::new();

    // TODO: parse image
    // TODO: parse devil abilities

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
            // TODO: age_selector

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
                Selector::parse(r#"div[data-source="affiliation"] > div > ul li"#).unwrap();
            let contract_selector =
                Selector::parse(r#"div[data-source="contracted humans"] > div"#).unwrap();
            let relative_selector =
                Selector::parse(r#"div[data-source="relatives"] > div"#).unwrap();

            // TODO: handle nested li elements
//             if let Some(div) = el.select(&occupation_selector).next() {
//                 let a_selector = Selector::parse(r#"a"#).unwrap();
//                 let nested_li_selector = Selector::parse(r#"ul > li"#).unwrap();

//                 let texts = div.text().collect::<Vec<_>>();
//                 for t in texts {
//                     occupations.push(t.trim().to_string());
//                 }
//             }

            for li in el.select(&affiliation_selector) {
                let texts = li.text().collect::<Vec<_>>();
                for t in texts {
                    affiliations.push(t.trim().to_string());
                }
            }

            if let Some(div) = el.select(&contract_selector).next() {
                let texts = div.text().collect::<Vec<_>>();
                for t in texts {
                    contracts.push(t.trim().to_string());
                }
            }
            if let Some(div) = el.select(&relative_selector).next() {
                let texts = div.text().collect::<Vec<_>>();
                for t in texts {
                    relatives.push(t.trim().to_string());
                }
            }
        }
    }

    // println!("occupations: {:?}", occupations);
    // println!("affiliations: {:?}", affiliations);
    // println!("contracts: {:?}", contracts);
    // println!("Relatives: {:?}", relatives);

    Ok(DevilDetail {
        names,
        image_src: None,
        gender,
        birthplace,
        age,
        status,
        occupations,
        affiliations,
        contracts,
        relatives,
    })
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // let _devils = match scrape_devils().await {
    //     Ok(devils) => devils,
    //     Err(e) => panic!("{}", e.to_string())
    // };

    if let Err(e) = scrape_devil_detail("https://chainsaw-man.fandom.com/wiki/Makima".into()).await
    {
        panic!("{}", e.to_string());
    }
}
