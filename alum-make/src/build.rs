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
    let segs: Vec<&str> = pattern.split('*').collect();
    let (first, last) = (segs[0], segs[segs.len() - 1]);
    let mut rest = rel;
    if !rest.starts_with(first) {
        return false;
    }
    rest = &rest[first.len()..];
    if !rest.ends_with(last) {
        return false;
    }
    let mid = &rest[..rest.len() - last.len()];
    let mut pos = 0;
    for seg in &segs[1..segs.len() - 1] {
        if seg.is_empty() {
            continue;
        }
        match mid[pos..].find(seg) {
            Some(i) => pos += i + seg.len(),
            None => return false,
        }
    }
    true
}

fn glob_sources(patterns: &[String]) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut out: Vec<PathBuf> = Vec::new();
    for pat in patterns {
        let mut found = Vec::new();
        if pat.contains('*') {
            for entry in WalkDir::new(".")
                .into_iter()
                .filter_entry(|e| {
                    !matches!(
                        e.file_name().to_str(),
                        Some(".git") | Some(".deps") | Some("target") | Some("node_modules")
                    )
                })
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file()
                    && entry.path().extension().and_then(|s| s.to_str()) == Some("c")
                {
                    let rel = entry
                        .path()
                        .strip_prefix(".")
                        .unwrap()
                        .to_string_lossy()
                        .into_owned();
                    if matches_pattern(&rel, pat) {
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
fn run_command(log: bool, cmd: &str, args: &[String], context: &str) -> Result<(), Box<dyn Error>> {
    let mut parts = cmd.split_whitespace().map(str::to_string);
    let Some(program) = parts.next() else {
        return Err(format!("{}: empty command", context).into());
    };
    let mut argv: Vec<String> = parts.collect();
    argv.extend_from_slice(args);
    if log {
        println!("{} {}", program, argv.join(" "));
    }
    let status = std::process::Command::new(&program).args(&argv).status()?;
    if !status.success() {
        return Err(format!("{} (command: {} {})", context, program, argv.join(" ")).into());
    }
    Ok(())
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

    let mut extra_args: Vec<String> = Vec::new();
    if let Some(flags) = &flags {
        extra_args.extend(flags.split_whitespace().map(str::to_string));
    }
    if let Some(includes) = &includes {
        for include in includes {
            extra_args.push(format!("-I{}", include));
        }
    }
    for file in source_files {
        let rel = file.strip_prefix("./").unwrap_or(file.as_path());
        let output = if rel.starts_with("src") {
            PathBuf::from("target/objects")
                .join(rel.strip_prefix("src").unwrap().with_extension("o"))
        } else if rel.starts_with("include") {
            PathBuf::from("target/objects").join(rel.with_extension("o"))
        } else {
            PathBuf::from("target/objects")
                .join(file.file_name().unwrap())
                .with_extension("o")
        };
        fs::create_dir_all(output.parent().unwrap())?;
        let mut args = extra_args.clone();
        args.push(file.to_string_lossy().into_owned());
        args.push("-c".to_owned());
        args.push("-o".to_owned());
        args.push(output.to_string_lossy().into_owned());
        run_command(
            log,
            &compiler,
            &args,
            &format!("Failed to compile file: {:?}", file),
        )?;
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
    let mut extra_args: Vec<String> = Vec::new();
    if let Some(flags) = &flags {
        extra_args.extend(flags.split_whitespace().map(str::to_string));
    }
    if let Some(includes) = &includes {
        for include in includes {
            extra_args.push(format!("-I{}", include));
        }
    }
    if source_files.is_empty() {
        return Ok(());
    }

    for file in source_files {
        let output = PathBuf::from("target/objects")
            .join(file.strip_prefix("./src").unwrap().with_extension("o"));
        fs::create_dir_all(output.parent().unwrap())?;

        let mut args = extra_args.clone();
        args.push(file.to_string_lossy().into_owned());
        args.push("-c".to_owned());
        args.push("-o".to_owned());
        args.push(output.to_string_lossy().into_owned());
        run_command(
            log,
            &compiler,
            &args,
            &format!("Failed to compile Alum file: {:?}", file),
        )?;
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

    let mut flags_args: Vec<String> = cflags
        .clone()
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_string)
        .collect();
    if let Some(includes) = &includes {
        for inc in includes {
            flags_args.push(format!("-I{}", inc));
        }
    }

    let mut obj_files: Vec<PathBuf> = Vec::new();
    for file in &sources {
        let rel = file.strip_prefix(".").unwrap_or(file.as_path());
        let out_name = rel
            .with_extension("")
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "_");
        let out = PathBuf::from("target/native")
            .join(out_name)
            .with_extension("o");
        fs::create_dir_all(out.parent().unwrap())?;
        let mut args = flags_args.clone();
        args.push(file.to_string_lossy().into_owned());
        args.push("-c".to_owned());
        args.push("-o".to_owned());
        args.push(out.to_string_lossy().into_owned());
        args.push("-fPIC".to_owned());
        run_command(
            log,
            &compiler,
            &args,
            &format!("Failed to compile native source: {:?}", file),
        )?;
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
        let mut args = vec!["rcs".to_owned(), path.to_string_lossy().into_owned()];
        for o in &obj_files {
            args.push(o.to_string_lossy().into_owned());
        }
        run_command(log, "ar", &args, "Failed to create native static library")?;
        Ok(Some(path))
    } else {
        let path = out_dir.join(format!("lib{}.so", lib_name));
        let mut args = vec!["-shared".to_owned(), "-fPIC".to_owned()];
        for o in &obj_files {
            args.push(o.to_string_lossy().into_owned());
        }
        args.push("-o".to_owned());
        args.push(path.to_string_lossy().into_owned());
        args.extend(flags_args.iter().cloned());
        run_command(
            log,
            &compiler,
            &args,
            "Failed to link native shared library",
        )?;
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
    if metadata(&"target/objects").is_ok() {
        fs::remove_dir_all("target/objects")?;
    }
    create_dir("target/objects")?;

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
                .map(|ab| ab.to_string_lossy().into_owned())
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
                let mut args = vec![format!("target/lib{}.a", name)];
                for file in &obj_files {
                    args.push(file.to_string_lossy().into_owned());
                }
                run_command(
                    log,
                    "ar rcs",
                    &args,
                    &format!("Failed to create static library: {:?}", name),
                )?;
            }
            Some("shared") | Some("so") => {
                let mut args = vec!["-shared".to_owned(), "-fPIC".to_owned()];
                for file in &obj_files {
                    args.push(file.to_string_lossy().into_owned());
                }
                args.push("-o".to_owned());
                args.push(format!("target/lib{}.so", name));
                run_command(
                    log,
                    &config.build.linker,
                    &args,
                    &format!("Failed to create shared library: {:?}", name),
                )?;
            }
            _ => {
                let mut args: Vec<String> = Vec::new();

                if config.build.linker.contains("cc") || config.build.linker.contains("gcc") {
                    args.push("-nostartfiles".to_owned());
                }

                if let Some(lnflags) = config.build.lnflags.clone() {
                    args.extend(lnflags.split_whitespace().map(str::to_string));
                }
                for file in &obj_files {
                    args.push(file.to_string_lossy().into_owned());
                }

                let should_link_stdlib =
                    config.build.alc.is_some() && config.build.nostdlib != Some(true);
                if should_link_stdlib {
                    args.push("/usr/local/lib/libalum.a".to_owned());
                }

                let is_alc_linker = config.build.linker == "alc";
                if let Some(ref artifact) = native_artifact {
                    let abs = artifact.canonicalize().unwrap_or_else(|_| artifact.clone());

                    if is_alc_linker {
                        args.push("--cte-lib".to_owned());
                        args.push(abs.to_string_lossy().into_owned());
                    } else {
                        if let Some(loader) = native_ld_loader() {
                            args.push("-dynamic-linker".to_owned());
                            args.push(loader.to_owned());
                        }
                        if let Some(parent) = abs.parent() {
                            args.push(format!("-Wl,-rpath,{}", parent.to_string_lossy()));
                        }
                        args.push(abs.to_string_lossy().into_owned());
                    }
                }

                args.push("-o".to_owned());
                args.push(format!("target/{}", name));
                run_command(
                    log,
                    &config.build.linker,
                    &args,
                    &format!("Failed to link file: {:?}", name),
                )?;
            }
        }
    }
    Ok(())
}
