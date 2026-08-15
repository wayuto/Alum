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
    if let Err(e) = run() {
        if let Some(ce) = e.downcast_ref::<CompilerError>() {
            eprint!("{}", ce.diagnose());
        } else {
            eprintln!("Error: {}", e);
        }
        exit(1);
    }
}

fn is_object_file(path: &str) -> bool {
    path.ends_with(".o") || path.ends_with(".obj") || path.ends_with(".a")
}

fn default_exe_path(input: &str) -> String {
    let name = input.strip_suffix(".al").unwrap_or(input);
    name.to_string()
}

fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let Some(first_input) = cli.input.first() else {
        return Err("No input files specified".into());
    };

    if cli.run {
        return exec_run(
            first_input.clone(),
            cli.include_paths,
            cli.verbose,
            cli.cte_lib.clone(),
        );
    }

    let all_obj_files = cli.input.iter().all(|input| is_object_file(input));

    let mut obj_files = Vec::new();
    let mut generated_objs = Vec::new();

    for input in &cli.input {
        if is_object_file(input) {
            obj_files.push(input.clone());
            continue;
        }

        let obj_output = if cli.compile_only {
            cli.output.clone()
        } else {
            None
        };

        let obj_file = build(
            input.clone(),
            cli.ast,
            obj_output,
            cli.include_paths.clone(),
            cli.preprocess_only,
            cli.verbose,
            cli.cte_lib.clone(),
        )?;

        if !obj_file.is_empty() {
            obj_files.push(obj_file.clone());
            generated_objs.push(obj_file);
        }
    }

    if let Some(lib_type) = cli.library.as_deref() {
        let output_path = cli
            .output
            .clone()
            .unwrap_or_else(|| format!("lib{}.a", default_exe_path(first_input)));

        match lib_type.to_lowercase().as_str() {
            "static" | "a" => create_static_library(obj_files, &output_path, cli.verbose)?,
            "shared" | "so" => create_shared_library(obj_files, &output_path, cli.verbose)?,
            _ => return Err(format!("Unknown library type: {}", lib_type).into()),
        }
        return Ok(());
    }

    if cli.compile_only || cli.ast || cli.preprocess_only {
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

    link(
        obj_files,
        &std_lib_path,
        &exe_path,
        cli.verbose,
        &cli.cte_lib,
    )?;

    for obj_file in generated_objs {
        if cli.verbose {
            eprintln!("Removing object file: {}", obj_file);
        }
        let _ = std::fs::remove_file(&obj_file);
    }

    Ok(())
}
