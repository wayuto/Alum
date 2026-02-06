mod cli;
mod compiler;
use clap::Parser;
use cli::{Cli, build, exec_run, link};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.input.is_empty() {
        eprintln!("Error: No input files specified");
        std::process::exit(1);
    }

    let mut obj_files = Vec::new();

    for input in &cli.input {
        if input.ends_with(".o") || input.ends_with(".obj") {
            obj_files.push(input.clone());
            continue;
        }

        let obj_file = build::build(
            input.clone(),
            cli.ast,
            None,
            cli.include_paths.clone(),
            cli.preprocess_only,
            cli.verbose,
        )?;

        if !obj_file.is_empty() {
            obj_files.push(obj_file);
        }
    }

    if cli.run {
        let input = cli.input.first().unwrap().clone();
        exec_run(input, cli.include_paths, cli.verbose)?;
        return Ok(());
    }

    if !cli.compile_only && !cli.ast && !cli.preprocess_only {
        let exe_path = if let Some(output) = cli.output {
            output
        } else {
            cli.input.first().unwrap().replace(".al", "")
        };

        let std_lib_path = if cli.nostdlib {
            String::new()
        } else {
            let path = "/usr/local/lib/libalum_std.a";
            path.to_string()
        };

        if cli.verbose {
            eprintln!("Linking {} to {}", obj_files.join(", "), exe_path);
        }

        link::link(obj_files, &std_lib_path, &exe_path, cli.verbose)?;
    }

    Ok(())
}
