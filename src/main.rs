mod cli;
mod compiler;

use clap::Parser;
use cli::{Cli, build, exec_run, link::link_objects};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.input.is_empty() {
        eprintln!("No input files specified");
        return Ok(());
    }

    let first_input = &cli.input[0];
    let extension = Path::new(first_input).extension().and_then(|e| e.to_str());

    let is_linking = extension == Some("o") || extension == Some("obj");

    if is_linking {
        let exe_name = cli.output.unwrap_or_else(|| "a.out".to_string());
        
        let std_lib_path = if cli.nostdlib {
            String::new()
        } else {
            std::env::var("ALUM_STD_PATH").unwrap_or_else(|_| {
                let project_root = std::env::var("CARGO_MANIFEST_DIR")
                    .unwrap_or_else(|_| ".".to_string());
                format!("{}/alum-std/target/release/libalum_std.a", project_root)
            })
        };

        if cli.verbose {
            eprintln!("Linking object files: {:?}", cli.input);
            if !cli.nostdlib {
                eprintln!("With standard library: {}", std_lib_path);
            }
        }

        link_objects(cli.input, &std_lib_path, &exe_name)?;
        return Ok(());
    }

    if cli.run {
        exec_run(first_input.clone(), cli.include_paths, cli.verbose)
    } else {
        let output = cli.output.clone();
        let obj_path = build(
            first_input.clone(),
            output.clone(),
            cli.compile_only,
            cli.ast,
            None,
            cli.include_paths,
            cli.preprocess_only,
            cli.verbose,
        )?;

        if !cli.compile_only && !cli.preprocess_only {
            if let Some(obj) = obj_path {
                let exe_name = output.unwrap_or_else(|| {
                    let path = std::path::PathBuf::from(first_input);
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("output")
                        .to_string()
                });
                
                let std_lib_path = if cli.nostdlib {
                    String::new()
                } else {
                    std::env::var("ALUM_STD_PATH").unwrap_or_else(|_| {
                        let project_root = std::env::var("CARGO_MANIFEST_DIR")
                            .unwrap_or_else(|_| ".".to_string());
                        format!("{}/alum-std/target/release/libalum_std.a", project_root)
                    })
                };

                if cli.verbose {
                    eprintln!("Linking...");
                    if !cli.nostdlib {
                        eprintln!("With standard library: {}", std_lib_path);
                    }
                }

                link_objects(vec![obj], &std_lib_path, &exe_name)?;
            }
        }

        Ok(())
    }
}
