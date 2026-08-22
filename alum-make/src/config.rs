use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub package: Package,
    pub build: Build,
    #[serde(default)]
    pub native: Option<Native>,
    pub dependencies: Option<HashMap<String, Dependency>>,
}

fn default_shared() -> bool {
    true
}

#[derive(Deserialize, Serialize)]
pub struct Native {
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default = "default_shared")]
    pub shared: bool,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub author: String,
    pub license: String,
    pub language: String,
}

#[derive(Deserialize, Serialize)]
pub struct Build {
    pub linker: String,
    pub cc: Option<String>,
    pub cflags: Option<String>,
    pub alc: Option<String>,
    pub alflags: Option<String>,
    pub lnflags: Option<String>,
    pub includes: Option<Vec<String>>,
    pub nostdlib: Option<bool>,
    #[serde(default)]
    pub library_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Dependency {
    pub local: Option<String>,
    pub url: Option<String>,
    pub git: bool,
    pub tag: Option<String>,
}
