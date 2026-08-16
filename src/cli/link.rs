use std::process::Command;

pub fn link(
    obj_files: Vec<String>,
    std_lib_path: &str,
    exe_path: &str,
    verbose: bool,
    native_libs: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new("lld");

    cmd.arg("-flavor").arg("gnu");
    cmd.arg("-O2");
    cmd.arg("-o").arg(exe_path);

    for obj_file in &obj_files {
        cmd.arg(obj_file);
    }

    if !std_lib_path.is_empty() {
        cmd.arg(std_lib_path);
    }

    for lib in native_libs {
        let abs = std::path::Path::new(lib)
            .canonicalize()
            .unwrap_or_else(|_| std::path::PathBuf::from(lib));
        if let Some(parent) = abs.parent() {
            cmd.arg("-rpath").arg(parent.to_str().unwrap().to_string());
        }
        cmd.arg(abs);
    }

    if !native_libs.is_empty() {
        let candidates = [
            "/lib64/ld-linux-x86-64.so.2",
            "/lib/ld-linux-x86-64.so.2",
            "/lib/ld-musl-x86_64.so.1",
        ];
        for loader in candidates.iter() {
            if std::path::Path::new(loader).exists() {
                cmd.arg("-dynamic-linker").arg(loader);
                break;
            }
        }
    }

    if verbose {
        eprintln!("Linking command: {:?}", cmd);
    }

    let output = match cmd.output() {
        Ok(output) => output,
        Err(e) => {
            return Err(format!("lld not found: {}. Please install lld.", e).into());
        }
    };

    if !output.status.success() {
        eprintln!("Linker error:");
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        return Err("Linking failed".into());
    }

    Ok(())
}

pub fn create_static_library(
    obj_files: Vec<String>,
    output_path: &str,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if verbose {
        eprintln!("Creating static library: {}", output_path);
    }

    let status = Command::new("ar")
        .arg("rcs")
        .arg(output_path)
        .args(&obj_files)
        .status()?;

    if !status.success() {
        return Err("Failed to create static library with 'ar' command. Please ensure ar is available (it's part of binutils on Linux/macOS, or available via MinGW on Windows).".into());
    }

    Ok(())
}

pub fn create_shared_library(
    obj_files: Vec<String>,
    output_path: &str,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if verbose {
        eprintln!("Creating shared library: {}", output_path);
    }

    let status = Command::new("lld")
        .arg("-flavor")
        .arg("gnu")
        .arg("-O2")
        .arg("-shared")
        .arg("-o")
        .arg(output_path)
        .args(&obj_files)
        .status()?;

    if !status.success() {
        return Err("Failed to create shared library with lld".into());
    }

    Ok(())
}
