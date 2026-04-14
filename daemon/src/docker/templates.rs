use once_cell::sync::Lazy;
use serde::Deserialize;
use std::{collections::HashMap, fs};

// So okay, this file is used to get the templates
// Templates are docker container images with an internal port defined that is premapped to the external port.
// You have templates.json file for that.
// There are all the templates defined and you can also easily add more.
// This makes sure everything is dynamic
// Soon I will add an api route for adding more templates
// Currently I will just optimise the code
// I gave up on the frontend....

#[derive(Debug, Deserialize, Clone)]
pub struct Template {
    pub name: String,
    pub image: String,
    pub default_internal_port: u16,
}

// Parse json
static TEMPLATE_MAP: Lazy<HashMap<String, Template>> = Lazy::new(|| {
    let data = fs::read_to_string("templates.json")
        .expect("Failed to read templates.json");

    let templates: Vec<Template> =
        serde_json::from_str(&data).expect("Invalid JSON");

    templates
        .into_iter()
        .map(|t| (t.name.clone(), t))
        .collect()
});

// Get the templates
#[inline]
pub fn get_template(name: &str) -> Option<&'static Template> {
    TEMPLATE_MAP.get(name)
}