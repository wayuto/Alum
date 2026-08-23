mod build;
mod command;
mod config;
mod dependencies;
mod new;
mod sync;
use std::fs::metadata;

use clap::Parser;
use command::{Arg, Commands};

use crate::{
    build::build,
    dependencies::{add_dep, rm_dep},
    new::new,
    sync::sync,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Arg::parse();

    match args.cmd {
        Commands::New { name, language } => match language.as_str() {
            "C" => {
                new(
                    name.clone(),
                    language.clone(),
                    Some("clang".to_owned()),
                    None,
                    "clang".to_owned(),
                )?;
                println!("'{}' has been created successfully.", name);
            }
            "alum" => {
                new(
                    name.clone(),
                    language.clone(),
                    None,
                    Some("alc".to_owned()),
                    "alc".to_owned(),
                )?;
                println!("'{}' has been created successfully.", name);
            }
            _ => {
                return Err(format!("Unsupported language: {:?}", language).into());
            }
        },
        Commands::Build => match build(true) {
            Ok(_) => {}
            Err(e) => return Err(e),
        },
        Commands::Clean => {
            if std::fs::metadata(&"target/").is_ok() {
                std::fs::remove_dir_all(&"target/")?;
                println!("Removed object files in 'target/' directory.");
            } else {
                println!("No 'target/' directory found to clean.");
            }

            if metadata(&".deps").is_ok() {
                std::fs::remove_dir_all(&".deps")?;
                println!("Removed dependencies in '.deps/' directory.");
            }
        }
        Commands::Run => {
            if let Err(e) = build(false) {
                return Err(e);
            }
            let toml_string = std::fs::read_to_string(&"./Alumake.toml")?;
            let config: config::Config = toml::from_str(&toml_string)?;

            if let Some(lib_type) = &config.build.library_type {
                if lib_type != "none" && lib_type != "executable" {
                    eprintln!("this is a library project; nothing to run");
                    std::process::exit(1);
                }
            }
            let name = config.package.name;
            let status = std::process::Command::new(format!("./target/{}", name)).status()?;
            if !status.success() {
                return Err(format!("Failed to run the executable: {:?}", name).into());
            }
        }
        Commands::Add {
            name,
            url,
            local,
            git,
            tag,
        } => match add_dep(name, local, url, git, tag) {
            Ok(_) => {}
            Err(e) => return Err(e),
        },
        Commands::Remove { name } => match rm_dep(name) {
            Ok(_) => {}
            Err(e) => return Err(e),
        },
        Commands::Sync => match sync() {
            Ok(_) => {}
            Err(e) => return Err(e),
        },
    }
    Ok(())
}
