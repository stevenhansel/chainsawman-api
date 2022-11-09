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

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(e) = scrape_devils().await {
        println!("{:?}", e);
    }
}
