use crate::config::Config;
use git2::Repository;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{File, copy, metadata, read_to_string},
    path::Path,
};
use zip::ZipArchive;

fn extract_zip(zip_path: &str, extract_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;
    std::fs::create_dir_all(extract_path)?;
    archive.extract(extract_path)?;
    Ok(())
}

fn get_hash(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let content = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

pub fn sync() -> Result<(), Box<dyn std::error::Error>> {
    let config: Config = toml::from_str(&read_to_string(&"./Alumake.toml")?)?;
    let mut deps_hash: HashMap<String, String> = match read_to_string("target/deps-sync.json") {
        Ok(content) => serde_json::from_str(&content)?,
        Err(_) => HashMap::new(),
    };

    if let Some(deps) = config.dependencies {
        for (name, info) in deps {
            if let Some(url) = info.url {
                if info.git {
                    match Repository::clone(&url, Path::new(&format!(".deps/{}", name))) {
                        Ok(_) => {}
                        Err(_) => {
                            if let Some(tag) = info.tag {
                                let repo = Repository::open(Path::new(&format!(".deps/{}", name)))?;
                                let (object, reference) = repo.revparse_ext(&tag)?;
                                repo.checkout_tree(&object, None)?;
                                match reference {
                                    Some(gref) => repo.set_head(gref.name().unwrap())?,
                                    None => repo.set_head_detached(object.id())?,
                                }
                            }
                        }
                    }
                } else {
                    let dep_dir = format!(".deps/{}", name);
                    let cached_hash = deps_hash.get(&name).map(|h| h.as_str());

                    if cached_hash.is_some() && Path::new(&dep_dir).exists() {
                        continue;
                    }

                    std::fs::create_dir_all(".deps")?;
                    let zip_path = format!(".deps/{}.zip", name);
                    println!("Downloading {}", name);
                    let response = ureq::get(&url).call()?;
                    let mut zip_file = File::create(&zip_path)?;
                    std::io::copy(&mut response.into_reader(), &mut zip_file)?;

                    let curr_hash = get_hash(&zip_path)?;

                    if cached_hash == Some(&curr_hash) && Path::new(&dep_dir).exists() {
                        std::fs::remove_file(&zip_path)?;
                    } else {
                        if Path::new(&dep_dir).exists() {
                            std::fs::remove_dir_all(&dep_dir)?;
                        }
                        extract_zip(&zip_path, &dep_dir)?;
                        std::fs::remove_file(&zip_path)?;
                        deps_hash.insert(name.clone(), curr_hash);
                    }
                }
            } else if let Some(local) = info.local {
                if metadata(&format!(".deps/{}", name)).is_err() {
                    copy(Path::new(&local), Path::new(&format!(".deps/{}", name)))?;
                    let curr_hash = get_hash(&local)?;
                    deps_hash.insert(name, curr_hash);
                }
            }
        }
    }

    std::fs::create_dir_all("target")?;
    serde_json::to_writer_pretty(std::fs::File::create("target/deps-sync.json")?, &deps_hash)?;
    Ok(())
}
