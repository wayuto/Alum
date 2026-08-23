use crate::config::Config;
use git2::Repository;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::{
    collections::HashMap,
    fs::{File, metadata, read_to_string},
    path::{Path, PathBuf},
};
use walkdir::WalkDir;
use zip::ZipArchive;

fn extract_zip(zip_path: &str, extract_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;
    std::fs::create_dir_all(extract_path)?;
    archive.extract(extract_path)?;
    Ok(())
}

fn get_hash(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut hasher = Sha256::new();
    let p = Path::new(path);
    if p.is_dir() {
        let mut files: Vec<PathBuf> = WalkDir::new(p)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_path_buf())
            .collect();
        files.sort();
        for file in files {
            hasher.update(file.to_string_lossy().as_bytes());
            hasher.update(&std::fs::read(&file)?);
        }
    } else {
        hasher.update(&std::fs::read(p)?);
    }
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

fn copy_dir(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let file_name = entry.file_name();
        let src_path = entry.path();
        let dst_path = dst.join(&file_name);
        if file_type.is_dir() {
            copy_dir(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn checkout_tag(repo: &Repository, tag: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (object, reference) = repo.revparse_ext(tag)?;
    repo.checkout_tree(&object, None)?;
    match reference {
        Some(gref) => repo.set_head(gref.name().unwrap())?,
        None => repo.set_head_detached(object.id())?,
    }
    Ok(())
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
                    let dest = format!(".deps/{}", name);
                    let dest_path = Path::new(&dest);

                    let repo = match Repository::open(dest_path) {
                        Ok(repo) => {
                            if let Ok(mut remote) = repo.find_remote("origin") {
                                let _ = remote.fetch(
                                    &["refs/heads/*:refs/remotes/origin/*"],
                                    None,
                                    None,
                                );
                            }
                            repo
                        }
                        Err(_) => {
                            if dest_path.exists() {
                                std::fs::remove_dir_all(dest_path)?;
                            }
                            std::fs::create_dir_all(".deps")?;
                            Repository::clone(&url, dest_path)?
                        }
                    };
                    if let Some(tag) = &info.tag {
                        checkout_tag(&repo, tag)?;
                    }
                } else {
                    let dep_dir = format!(".deps/{}", name);

                    if deps_hash.contains_key(&url) && Path::new(&dep_dir).exists() {
                        continue;
                    }

                    std::fs::create_dir_all(".deps")?;
                    let zip_path = format!(".deps/{}.zip", name);
                    println!("Downloading {}", name);
                    let response = ureq::get(&url).call()?;

                    const MAX_ZIP_BYTES: u64 = 512 * 1024 * 1024;
                    if let Some(len) = response.header("Content-Length") {
                        if let Ok(n) = len.parse::<u64>() {
                            if n > MAX_ZIP_BYTES {
                                return Err(format!(
                                    "download for '{}' too large: {} bytes",
                                    name, n
                                )
                                .into());
                            }
                        }
                    }
                    let mut zip_file = File::create(&zip_path)?;
                    let mut reader = response.into_reader();
                    let mut copied: u64 = 0;
                    loop {
                        use std::io::Read as _;
                        let mut buf = [0u8; 64 * 1024];
                        let n = reader.read(&mut buf)?;
                        if n == 0 {
                            break;
                        }
                        copied += n as u64;
                        if copied > MAX_ZIP_BYTES {
                            return Err(format!(
                                "download for '{}' exceeded {} bytes",
                                name, MAX_ZIP_BYTES
                            )
                            .into());
                        }
                        zip_file.write_all(&buf[..n])?;
                    }

                    let curr_hash = get_hash(&zip_path)?;

                    if Path::new(&dep_dir).exists() {
                        std::fs::remove_dir_all(&dep_dir)?;
                    }
                    extract_zip(&zip_path, &dep_dir)?;
                    std::fs::remove_file(&zip_path)?;
                    deps_hash.insert(url.clone(), curr_hash);
                }
            } else if let Some(local) = info.local {
                let dest = format!(".deps/{}", name);
                let curr_hash = get_hash(&local)?;
                let up_to_date = deps_hash.get(&name).map(|h| h.as_str())
                    == Some(curr_hash.as_str())
                    && metadata(&dest).is_ok();
                if !up_to_date {
                    let src = Path::new(&local);
                    let dst = Path::new(&dest);
                    if metadata(&dest).is_ok() {
                        if dst.is_dir() {
                            std::fs::remove_dir_all(dst)?;
                        } else {
                            std::fs::remove_file(dst)?;
                        }
                    }
                    if src.is_dir() {
                        copy_dir(src, dst)?;
                    } else {
                        if let Some(parent) = dst.parent() {
                            std::fs::create_dir_all(parent)?;
                        }
                        std::fs::copy(src, dst)?;
                    }
                    deps_hash.insert(name, curr_hash);
                }
            }
        }
    }

    std::fs::create_dir_all("target")?;
    serde_json::to_writer_pretty(std::fs::File::create("target/deps-sync.json")?, &deps_hash)?;
    Ok(())
}
