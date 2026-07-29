mod cli;
mod compiler;

use clap::Parser;
use cli::{
    build::DEFAULT_STD_LIB_PATH,
    link::{create_shared_library, create_static_library, link},
    {Cli, build, exec_run},
};

fn main() {
    if let Err(e) = run() {
        if let Some(ce) = e.downcast_ref::<compiler::CompilerError>() {
            eprint!("{}", ce.diagnose());
        } else {
            eprintln!("Error: {}", e);
        }
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.input.is_empty() {
        eprintln!("Error: No input files specified");
        std::process::exit(1);
    }

    if cli.run {
        let input = cli.input.first().unwrap().clone();
        exec_run(input, cli.include_paths, cli.verbose)?;
        return Ok(());
    }

    let mut obj_files = Vec::new();

    let all_obj_files = cli
        .input
        .iter()
        .all(|input| input.ends_with(".o") || input.ends_with(".obj") || input.ends_with(".a"));

    for input in &cli.input {
        if input.ends_with(".o") || input.ends_with(".obj") || input.ends_with(".a") {
            obj_files.push(input.clone());
            continue;
        }

        let obj_output = if cli.compile_only {
            cli.output.clone()
        } else {
            None
        };

        let obj_file = build::build(
            input.clone(),
            cli.ast,
            obj_output,
            cli.include_paths.clone(),
            cli.preprocess_only,
            cli.verbose,
        )?;

        if !obj_file.is_empty() {
            obj_files.push(obj_file);
        }
    }

    if all_obj_files && !cli.compile_only && !cli.ast && !cli.preprocess_only {
        let exe_path = if let Some(output) = cli.output {
            output
        } else {
            "a.out".to_string()
        };

        let std_lib_path = if cli.nostdlib {
            String::new()
        } else {
            let has_stdlib = obj_files.iter().any(|f| f.contains("libalum.a"));
            if has_stdlib {
                String::new()
            } else {
                DEFAULT_STD_LIB_PATH.to_string()
            }
        };

        if cli.verbose {
            eprintln!("Linking {} to {}", obj_files.join(", "), exe_path);
        }

        link(obj_files, &std_lib_path, &exe_path, cli.verbose)?;
        return Ok(());
    }

    if cli.library.is_some() {
        let lib_type = cli.library.as_ref().unwrap().to_lowercase();
        let output_path = if let Some(output) = cli.output {
            output
        } else {
            let first_input = cli.input.first().unwrap();
            let name = if first_input.ends_with(".al") {
                first_input.replace(".al", "")
            } else {
                first_input.clone()
            };
            format!("lib{}.a", name)
        };

        match lib_type.as_str() {
            "static" | "a" => {
                create_static_library(obj_files, &output_path, cli.verbose)?;
            }
            "shared" | "so" => {
                create_shared_library(obj_files, &output_path, cli.verbose)?;
            }
            _ => {
                return Err(format!("Unknown library type: {}", lib_type).into());
            }
        }
        return Ok(());
    }

    if !cli.compile_only && !cli.ast && !cli.preprocess_only {
        let exe_path = if let Some(output) = cli.output {
            output
        } else {
            let first_input = cli.input.first().unwrap();
            if first_input.ends_with(".al") {
                first_input.replace(".al", "")
            } else {
                first_input.clone()
            }
        };

        let std_lib_path = if cli.nostdlib {
            String::new()
        } else {
            DEFAULT_STD_LIB_PATH.to_string()
        };

        if cli.verbose {
            eprintln!("Linking {} to {}", obj_files.join(", "), exe_path);
        }

        link(obj_files, &std_lib_path, &exe_path, cli.verbose)?;
    }

    Ok(())
}
