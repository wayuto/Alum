use std::process::Command;

pub fn link(
    obj_files: Vec<String>,
    std_lib_path: &str,
    exe_path: &str,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new("rust-lld");

    cmd.arg("-flavor").arg("gnu");

    cmd.arg("-o").arg(exe_path);

    for obj_file in &obj_files {
        cmd.arg(obj_file);
    }

    if !std_lib_path.is_empty() {
        cmd.arg(std_lib_path);
    }

    if verbose {
        eprintln!("Linking command: {:?}", cmd);
    }

    let output = match cmd.output() {
        Ok(output) => output,
        Err(e) => {
            return Err(format!("rust-lld not found: {}. Please install rust-lld.", e).into());
        }
    };

    if !output.status.success() {
        eprintln!("Linker error:");
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        return Err("Linking failed".into());
    }

    for obj_file in obj_files {
        if obj_file.ends_with(".o") || obj_file.ends_with(".obj") {
            if verbose {
                eprintln!("Removing object file: {}", obj_file);
            }
            let _ = std::fs::remove_file(&obj_file);
        }
    }

    Ok(())
}
