use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Devil {
    pub devil_name: String,
    pub alias_name: Option<String>,
    pub wiki_url: String,
    pub category: String,
}

#[derive(Debug, Clone)]
pub struct DevilDetail {
    pub names: HashMap<String, DevilName>,
    pub image_src: Option<String>,
    pub gender: Option<String>,
    pub birthplace: Option<String>,
    pub status: Option<String>,
    pub occupations: Vec<String>,
    pub affiliations: Vec<String>,
    pub contracts: Vec<String>,
    pub relatives: Vec<String>,
    pub abilities: HashMap<String, Vec<Ability>>,
}

/**
* Names will be stored as a hashmap, where the key is the language code
* */
#[derive(Debug, Clone)]
pub struct DevilName {
    pub devil_name: String,
    pub alias_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Ability {
    pub name: String,
    pub description: String,
    pub abilities: Vec<Ability>,
}
