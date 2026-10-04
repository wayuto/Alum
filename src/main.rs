mod cli;

use alc::compiler::CompilerError;
use clap::Parser;
use cli::{
    build::DEFAULT_STD_LIB_PATH,
    link::{create_shared_library, create_static_library, link},
    {Cli, build, exec_run},
};
use std::{error::Error, process::exit};

fn main() {
    let handle = std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(|| -> Result<(), String> {
            run().map_err(|e| {
                if let Some(ce) = e.downcast_ref::<CompilerError>() {
                    ce.diagnose()
                } else {
                    format!("Error: {}", e)
                }
            })
        })
        .expect("failed to spawn compiler thread");
    match handle.join() {
        Ok(Ok(())) => {}
        Ok(Err(diag)) => {
            eprint!("{}", diag);
            exit(1);
        }
        Err(_) => {
            eprintln!("internal error: compiler thread panicked");
            exit(101);
        }
    }
}

fn is_object_file(path: &str) -> bool {
    path.ends_with(".o") || path.ends_with(".obj") || path.ends_with(".a")
}

fn default_exe_path(input: &str) -> String {
    let name = input.strip_suffix(".al").unwrap_or(input);
    name.to_string()
}

fn default_obj_path(input: &str) -> String {
    let stem = std::path::Path::new(input)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    format!("{}.o", stem)
}

fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let Some(first_input) = cli.input.first() else {
        return Err("No input files specified".into());
    };

    if cli.run {
        if cli.input.len() > 1 {
            return Err("'-r' accepts a single input file".into());
        }
        return exec_run(
            first_input.clone(),
            cli.output.clone(),
            cli.nostdlib,
            cli.include_paths,
            cli.verbose,
            cli.cte_lib.clone(),
            alc::compiler::codegen::DumpOptions {
                ir: cli.emit_ir,
                asm: cli.emit_asm,
            },
        );
    }

    if cli.compile_only && cli.input.len() > 1 && cli.output.is_some() {
        return Err("cannot specify '-o' with '-c' and multiple input files".into());
    }

    let all_obj_files = cli.input.iter().all(|input| is_object_file(input));

    let dumps = alc::compiler::codegen::DumpOptions {
        ir: cli.emit_ir,
        asm: cli.emit_asm,
    };
    let mut obj_files = Vec::new();
    let mut generated_objs = Vec::new();

    for input in &cli.input {
        if is_object_file(input) {
            obj_files.push(input.clone());
            continue;
        }

        let obj_output = if cli.compile_only {
            match cli.output.clone() {
                Some(o) => Some(o),
                None => {
                    if cli.input.len() > 1 {
                        Some(input.trim_end_matches(".al").replace('/', "_") + ".o")
                    } else {
                        Some(default_obj_path(input))
                    }
                }
            }
        } else {
            None
        };

        let obj_file = build(
            input.clone(),
            cli.emit_ast,
            obj_output,
            cli.include_paths.clone(),
            cli.preprocess_only,
            cli.verbose,
            cli.cte_lib.clone(),
            dumps,
        )?;

        if !obj_file.is_empty() {
            obj_files.push(obj_file.clone());
            generated_objs.push(obj_file);
        }
    }

    if let Some(lib_type) = cli.library.as_deref() {
        let output_path = cli.output.clone().unwrap_or_else(|| {
            let base = std::path::Path::new(first_input)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("a")
                .to_string();
            match lib_type.to_lowercase().as_str() {
                "shared" | "so" => format!("lib{}.so", base),
                _ => format!("lib{}.a", base),
            }
        });

        match lib_type.to_lowercase().as_str() {
            "static" | "a" => create_static_library(obj_files, &output_path, cli.verbose)?,
            "shared" | "so" => create_shared_library(obj_files, &output_path, cli.verbose)?,
            _ => return Err(format!("Unknown library type: {}", lib_type).into()),
        }
        return Ok(());
    }

    if cli.compile_only || cli.emit_ast || cli.preprocess_only {
        return Ok(());
    }

    let exe_path = cli
        .output
        .clone()
        .unwrap_or_else(|| default_exe_path(first_input));

    let std_lib_path = if cli.nostdlib {
        String::new()
    } else if all_obj_files && obj_files.iter().any(|f| f.contains("libalum.a")) {
        String::new()
    } else {
        DEFAULT_STD_LIB_PATH.to_string()
    };

    if cli.verbose {
        eprintln!("Linking {} to {}", obj_files.join(", "), exe_path);
    }

    let link_result = link(
        obj_files,
        &std_lib_path,
        &exe_path,
        cli.verbose,
        &cli.cte_lib,
    );

    for obj_file in generated_objs {
        if cli.verbose {
            eprintln!("Removing object file: {}", obj_file);
        }
        let _ = std::fs::remove_file(&obj_file);
    }

    link_result.map(|_| ())
}
