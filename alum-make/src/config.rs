use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub package: Package,
    pub build: Build,
    pub dependencies: Option<HashMap<String, Dependency>>,
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
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Dependency {
    pub local: Option<String>,
    pub url: Option<String>,
    pub git: bool,
    pub tag: Option<String>,
}
