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

fn matches_pattern(rel: &str, pattern: &str) -> bool {
    if !pattern.contains('*') {
        return rel == pattern;
    }
    let (pre, post) = pattern.split_once('*').unwrap();
    if let Some(rest) = rel.strip_prefix(pre) {
        if let Some(stripped) = rest.strip_suffix(post) {
            return !stripped.is_empty();
        }
    }
    false
}

fn glob_sources(patterns: &[String]) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut out: Vec<PathBuf> = Vec::new();
    for pat in patterns {
        let mut found = Vec::new();
        if pat.contains('*') {
            for entry in WalkDir::new(".").into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file()
                    && entry.path().extension().and_then(|s| s.to_str()) == Some("c")
                {
                    let rel = entry.path().strip_prefix(".").unwrap().to_str().unwrap();
                    if matches_pattern(rel, pat) {
                        found.push(entry.path().to_path_buf());
                    }
                }
            }
        } else {
            let p = PathBuf::from(pat);
            if metadata(&p).is_ok() {
                found.push(p);
            }
        }
        if found.is_empty() {
            return Err(format!("native source pattern '{}' did not match any file", pat).into());
        }
        out.extend(found);
    }
    Ok(out)
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

fn build_native(
    log: bool,
    cc: &Option<String>,
    cflags: &Option<String>,
    includes: Option<Vec<String>>,
    native: &crate::config::Native,
    name: &str,
) -> Result<Option<PathBuf>, Box<dyn Error>> {
    let compiler = match cc {
        Some(c) => c.clone(),
        None => return Ok(None),
    };

    let sources = glob_sources(&native.sources)?;
    if sources.is_empty() {
        return Ok(None);
    }

    let mut flags = cflags.clone().unwrap_or_default();
    if let Some(includes) = includes {
        for inc in includes {
            flags.push_str(&format!(" -I{}", inc));
        }
    }

    let mut obj_files: Vec<PathBuf> = Vec::new();
    for file in &sources {
        let out = PathBuf::from("target/native")
            .join(file.file_stem().unwrap().to_str().unwrap())
            .with_extension("o");
        fs::create_dir_all(out.parent().unwrap())?;
        let mut cmd = compiler.clone();
        cmd.push_str(" ");
        cmd.push_str(file.to_str().unwrap());
        cmd.push_str(" -c -o ");
        cmd.push_str(out.to_str().unwrap());
        if !flags.is_empty() {
            cmd.push_str(" ");
            cmd.push_str(&flags);
        }
        cmd.push_str(" -fPIC");
        if log {
            println!("{}", cmd);
        }
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .status()?;
        if !status.success() {
            return Err(format!("Failed to compile native source: {:?}", file).into());
        }
        obj_files.push(out);
    }

    let lib_name = native
        .name
        .clone()
        .unwrap_or_else(|| format!("{}_native", name));
    let out_dir = std::env::current_dir()?.join("target");
    fs::create_dir_all(&out_dir)?;

    if !native.shared {
        let path = out_dir.join(format!("lib{}.a", lib_name));
        let mut ar = String::from("ar rcs ");
        ar.push_str(path.to_str().unwrap());
        ar.push(' ');
        for o in &obj_files {
            ar.push_str(o.to_str().unwrap());
            ar.push(' ');
        }
        if log {
            println!("{}", ar);
        }
        let s = std::process::Command::new("sh")
            .arg("-c")
            .arg(&ar)
            .status()?;
        if !s.success() {
            return Err("Failed to create native static library".into());
        }
        Ok(Some(path))
    } else {
        let path = out_dir.join(format!("lib{}.so", lib_name));
        let mut link = compiler.clone();
        link.push_str(" -shared -fPIC");
        for o in &obj_files {
            link.push(' ');
            link.push_str(o.to_str().unwrap());
        }
        link.push_str(&format!(" -o {}", path.display()));
        if !flags.is_empty() {
            link.push_str(" ");
            link.push_str(&flags);
        }
        if log {
            println!("{}", link);
        }
        let s = std::process::Command::new("sh")
            .arg("-c")
            .arg(&link)
            .status()?;
        if !s.success() {
            return Err("Failed to link native shared library".into());
        }
        Ok(Some(path))
    }
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

    let native_artifact = if let Some(ref native) = config.native {
        build_native(
            log,
            &config.build.cc,
            &config.build.cflags,
            config.build.includes.clone(),
            native,
            &name,
        )?
    } else {
        None
    };

    let native_cte_arg = native_artifact.as_ref().and_then(|p| {
        if p.extension().and_then(|e| e.to_str()) == Some("so") {
            p.canonicalize()
                .ok()
                .map(|ab| ab.to_str().unwrap().to_owned())
        } else {
            None
        }
    });

    if let Some(ref cc) = config.build.cc {
        compile_c(
            log,
            cc.clone(),
            config.build.cflags.clone(),
            config.build.includes.clone(),
        )?;
    }

    if let Some(ref alc) = config.build.alc {
        let mut alflags = config.build.alflags.clone();
        if let Some(ref cte) = native_cte_arg {
            let extra = format!(" --cte-lib {}", cte);
            match &mut alflags {
                Some(f) => {
                    if !f.contains("--cte-lib") {
                        f.push_str(&extra);
                    }
                }
                None => alflags = Some(extra.trim_start().to_owned()),
            }
        }
        compile_alum(log, alc.clone(), alflags, config.build.includes.clone())?;
    }

    let obj_files = get_files("target/objects", Target::OBJ);

    if !obj_files.is_empty() {
        let native_ld_loader = || -> Option<&'static str> {
            for p in [
                "/lib64/ld-linux-x86-64.so.2",
                "/lib/ld-linux-x86-64.so.2",
                "/lib/ld-musl-x86_64.so.1",
            ] {
                if metadata(p).is_ok() {
                    return Some(p);
                }
            }
            None
        };

        match config.build.library_type.as_deref() {
            Some("static") | Some("a") => {
                let mut ar_cmd = String::from("ar rcs ");
                ar_cmd.push_str(&format!("target/lib{}.a ", name));
                for file in &obj_files {
                    ar_cmd.push_str(file.to_str().unwrap());
                    ar_cmd.push(' ');
                }
                if log {
                    println!("{}", ar_cmd);
                }
                let status = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(ar_cmd)
                    .status()?;
                if !status.success() {
                    return Err(format!("Failed to create static library: {:?}", name).into());
                }
            }
            Some("shared") | Some("so") => {
                let mut link_cmd = config.build.linker.clone();
                link_cmd.push_str(" -shared -fPIC ");
                for file in &obj_files {
                    link_cmd.push_str(file.to_str().unwrap());
                    link_cmd.push(' ');
                }
                link_cmd.push_str(&format!("-o target/lib{}.so", name));
                if log {
                    println!("{}", link_cmd);
                }
                let status = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(link_cmd)
                    .status()?;
                if !status.success() {
                    return Err(format!("Failed to create shared library: {:?}", name).into());
                }
            }
            _ => {
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

                let should_link_stdlib =
                    config.build.alc.is_some() && config.build.nostdlib != Some(true);
                if should_link_stdlib {
                    link_cmd.push_str("/usr/local/lib/libalum.a ");
                }

                let is_alc_linker = config.build.linker == "alc";
                if let Some(ref artifact) = native_artifact {
                    let abs = artifact.canonicalize().unwrap_or_else(|_| artifact.clone());

                    if is_alc_linker {
                        link_cmd.push_str("--cte-lib ");
                        link_cmd.push_str(abs.to_str().unwrap());
                        link_cmd.push(' ');
                    } else {
                        if let Some(loader) = native_ld_loader() {
                            link_cmd.push_str("-dynamic-linker ");
                            link_cmd.push_str(loader);
                            link_cmd.push(' ');
                        }
                        if let Some(parent) = abs.parent() {
                            link_cmd.push_str(&format!("-Wl,-rpath,{} ", parent.to_str().unwrap()));
                        }
                        link_cmd.push_str(&format!("{} ", abs.to_str().unwrap()));
                    }
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
        }
    }
    Ok(())
}
