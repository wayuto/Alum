use crate::{config::Config, sync::sync};
use std::{
    error::Error,
    fs::{self, create_dir, metadata},
    path::PathBuf,
};
use walkdir::WalkDir;

pub enum Target {
    CSRC,
    ALSRC,
    OBJ,
}

static mut LINK: bool = false;

fn get_files(root: &str, target: Target) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_path_buf())
        .filter(|p| match target {
            Target::CSRC => matches!(p.extension().and_then(|s| s.to_str()), Some("c")),
            Target::ALSRC => matches!(p.extension().and_then(|s| s.to_str()), Some("al")),
            Target::OBJ => matches!(p.extension().and_then(|s| s.to_str()), Some("o")),
        })
        .collect()
}

fn compile_c(
    log: bool,
    compiler: String,
    flags: Option<String>,
    includes: Option<Vec<String>>,
) -> Result<(), Box<dyn Error>> {
    let mut source_files = get_files("./src", Target::CSRC);

    if let Some(ref includes) = includes {
        for include_path in includes {
            if metadata(include_path).is_ok() {
                let include_files = get_files(include_path, Target::CSRC);
                source_files.extend(include_files);
            }
        }
    }

    let mut flags = if let Some(flags) = flags {
        flags
    } else {
        String::new()
    };
    if let Some(includes) = includes {
        for include in includes {
            flags.push_str(&format!(" -I{}", include));
        }
    }
    for file in source_files {
        let output = if file.starts_with("./src") {
            PathBuf::from("target/objects")
                .join(file.strip_prefix("./src").unwrap().with_extension("o"))
        } else if file.starts_with("./include") {
            PathBuf::from("target/objects")
                .join(file.strip_prefix(".").unwrap().with_extension("o"))
        } else {
            PathBuf::from("target/objects")
                .join(file.file_name().unwrap())
                .with_extension("o")
        };
        fs::create_dir_all(output.parent().unwrap())?;
        unsafe {
            LINK = true;
        }
        let mut cmd = compiler.clone();
        cmd.push_str(" ");
        cmd.push_str(file.to_str().unwrap());
        cmd.push_str(" -c -o ");
        cmd.push_str(output.to_str().unwrap());
        if !flags.is_empty() {
            cmd.push_str(" ");
            cmd.push_str(&flags);
        }
        if log {
            println!("{}", cmd);
        }
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .status()?;
        if !status.success() {
            return Err(format!("Failed to compile file: {:?}", file).into());
        }
    }
    Ok(())
}

fn compile_alum(
    log: bool,
    compiler: String,
    flags: Option<String>,
    includes: Option<Vec<String>>,
    output_name: &str,
) -> Result<(), Box<dyn Error>> {
    let source_files = get_files("./src", Target::ALSRC);
    let mut flags = if let Some(flags) = flags {
        flags
    } else {
        String::new()
    };
    if let Some(includes) = includes {
        for include in includes {
            flags.push_str(&format!(" -I{}", include));
        }
    }
    if source_files.is_empty() {
        return Ok(());
    }

    unsafe {
        LINK = true;
    }

    for file in source_files {
        let output = PathBuf::from("target/objects")
            .join(file.strip_prefix("./src").unwrap().with_extension("o"));
        fs::create_dir_all(output.parent().unwrap())?;

        let mut cmd = compiler.clone();
        cmd.push_str(" ");
        cmd.push_str(file.to_str().unwrap());
        cmd.push_str(" -c -o ");
        cmd.push_str(output.to_str().unwrap());
        if !flags.is_empty() {
            cmd.push_str(" ");
            cmd.push_str(&flags);
        }
        if log {
            println!("{}", cmd);
        }
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .status()?;
        if !status.success() {
            return Err(format!("Failed to compile Alum file: {:?}", file).into());
        }
    }
    Ok(())
}

pub fn build(log: bool) -> Result<(), Box<dyn Error>> {
    sync()?;
    let toml_string = fs::read_to_string(&"./Alumake.toml")?;
    let config: Config = toml::from_str(&toml_string)?;
    if metadata(&"target").is_err() {
        create_dir(&"target")?;
    }
    if metadata(&"target/objects").is_err() {
        create_dir(&"target/objects")?;
    }

    let name = config.package.name.clone();

    if let Some(ref cc) = config.build.cc {
        compile_c(
            log,
            cc.clone(),
            config.build.cflags.clone(),
            config.build.includes.clone(),
        )?;
    }

    if let Some(ref alc) = config.build.alc {
        compile_alum(
            log,
            alc.clone(),
            config.build.alflags.clone(),
            config.build.includes.clone(),
            &name,
        )?;
    }

    let obj_files = get_files("target/objects", Target::OBJ);

    if !obj_files.is_empty() {
        let mut link_cmd = config.build.linker.clone();
        link_cmd.push(' ');

        if config.build.linker.contains("cc") || config.build.linker.contains("gcc") {
            link_cmd.push_str("-nostartfiles ");
        }

        if let Some(lnflags) = config.build.lnflags.clone() {
            link_cmd.push_str(&lnflags);
            link_cmd.push(' ');
        }
        for file in &obj_files {
            link_cmd.push_str(file.to_str().unwrap());
            link_cmd.push(' ');
        }

        let should_link_stdlib = config.build.alc.is_some() && config.build.nostdlib != Some(true);
        if should_link_stdlib {
            link_cmd.push_str("/usr/local/lib/libalum_std.a ");
        }
        link_cmd.push_str(&format!("-o target/{}", name));
        if log {
            println!("{}", link_cmd);
        }
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(link_cmd)
            .status()?;
        if !status.success() {
            return Err(format!("Failed to link file: {:?}", name).into());
        }
    }
    Ok(())
}
