use std::{
    collections::HashMap,
    fs::{create_dir, metadata, read_to_string},
};

use crate::config::{Config, Dependency};

pub fn add_dep(
    name: String,
    local: Option<String>,
    url: Option<String>,
    git: bool,
    tag: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config: Config = toml::from_str(&read_to_string(&"./Alumake.toml")?)?;
    if let Some(ref mut deps) = config.dependencies {
        deps.insert(
            name.clone(),
            Dependency {
                local,
                url,
                git,
                tag,
            },
        );
    } else {
        let mut deps = HashMap::new();
        deps.insert(
            name.clone(),
            Dependency {
                local,
                url,
                git,
                tag,
            },
        );
        config.dependencies = Some(deps);
    }
    if metadata(".deps/".to_string()).is_err() {
        create_dir(".deps/")?;
    }
    std::fs::write("./Alumake.toml", toml::to_string_pretty(&config)?)?;
    println!("Added dependency '{}'", name);
    Ok(())
}

pub fn rm_dep(name: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut config: Config = toml::from_str(&read_to_string(&"./Alumake.toml")?)?;
    if let Some(ref mut deps) = config.dependencies {
        if deps.contains_key(&name) {
            deps.remove(&name);
        } else {
            return Err(format!("No dependency {} found", name).into());
        }
        if deps.is_empty() {
            config.dependencies = None;
        }
    } else {
        return Err(format!("No dependency '{}' found", name).into());
    }
    std::fs::write("./Alumake.toml", toml::to_string_pretty(&config)?)?;
    Ok(())
}
