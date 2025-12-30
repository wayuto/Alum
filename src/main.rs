#![allow(warnings)]
use crate::codegen::CodeGen;
use crate::irgen::IRGen;
use crate::{lexer::Lexer, parser::Parser, preprocessor::Preprocessor};
use clap::{Arg, ArgAction, Command};
use std::{fs, path::Path};

pub mod ast;
pub mod codegen;
pub mod ir;
pub mod irgen;
pub mod lexer;
pub mod parser;
pub mod preprocessor;
pub mod token;

fn print_ast(file: &String) -> Result<(), Box<dyn std::error::Error>> {
    let src = fs::read_to_string(file)?;
    let path = Path::new(&file.clone())
        .parent()
        .ok_or("Invalid file path")?
        .to_str()
        .ok_or("Invalid path encoding")?
        .to_string();
    let mut preprocessor = Preprocessor::new(src.as_str(), path);
    let code = preprocessor.preprocess()?;
    let lexer = Lexer::new(code.as_str());
    let mut parser = Parser::new(lexer);
    let ast = parser.parse()?;
    println!("{:#?}", ast);
    Ok(())
}

fn print_ir(file: &String) -> Result<(), Box<dyn std::error::Error>> {
    let src = fs::read_to_string(file)?;
    let path = Path::new(&file.clone())
        .parent()
        .ok_or("Invalid file path")?
        .to_str()
        .ok_or("Invalid path encoding")?
        .to_string();
    let mut preprocessor = Preprocessor::new(src.as_str(), path);
    let code = preprocessor.preprocess()?;
    let lexer = Lexer::new(code.as_str());
    let mut parser = Parser::new(lexer);
    let ast = parser.parse()?;
    let mut irgen = IRGen::new();
    let ir = irgen.compile(ast)?;
    println!("{:#?}", ir);
    Ok(())
}

fn print_pred(file: &String) -> Result<(), Box<dyn std::error::Error>> {
    let src = fs::read_to_string(file)?;
    let path = Path::new(&file.clone())
        .parent()
        .ok_or("Invalid file path")?
        .to_str()
        .ok_or("Invalid path encoding")?
        .to_string();
    let mut preprocessor = Preprocessor::new(src.as_str(), path);
    let code = preprocessor.preprocess()?;
    println!("{}", code);
    Ok(())
}

fn compile(
    input_file: &str,
    output_file: Option<&str>,
    emit_type: &str,
    no_std: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let src = fs::read_to_string(input_file)?;
    let path = Path::new(&input_file)
        .parent()
        .ok_or("Invalid file path")?
        .to_str()
        .ok_or("Invalid path encoding")?
        .to_string();
    let mut preprocessor = Preprocessor::new(&src, path);
    let code = preprocessor.preprocess()?;
    let lexer = Lexer::new(&code);
    let mut parser = Parser::new(lexer);
    let ast = parser.parse()?;
    let mut irgen = IRGen::new();
    let ir = irgen.compile(ast)?;
    let mut codegen = CodeGen::new(ir);
    let assembly = codegen.compile()?;

    let input_path = Path::new(input_file);
    let stem = input_path
        .file_stem()
        .ok_or("Invalid input filename")?
        .to_str()
        .ok_or("Invalid filename encoding")?;

    let output = if let Some(output_path) = output_file {
        output_path.to_string()
    } else {
        match emit_type {
            "asm" => format!("{}.s", stem),
            "obj" => format!("{}.o", stem),
            "bin" => stem.to_string(),
            _ => stem.to_string(),
        }
    };

    match emit_type {
        "asm" => {
            fs::write(&output, &assembly)?;
        }
        "obj" => {
            let asm_file = format!("{}.s", output);
            fs::write(&asm_file, &assembly)?;

            let nasm_status = std::process::Command::new("nasm")
                .args(&["-f", "elf64", "-o", &output, &asm_file])
                .status()?;

            if !nasm_status.success() {
                let _ = fs::remove_file(&asm_file);
                return Err("nasm failed".into());
            }

            let _ = fs::remove_file(&asm_file);
        }
        "bin" => {
            let asm_file = format!("{}.asm", output);
            let obj_file = format!("{}.o", output);

            fs::write(&asm_file, &assembly)?;

            let nasm_status = std::process::Command::new("nasm")
                .args(&["-f", "elf64", "-o", &obj_file, &asm_file])
                .status()?;

            if !nasm_status.success() {
                let _ = fs::remove_file(&asm_file);
                return Err("nasm failed".into());
            }

            let mut ld_args = vec!["-o", &output, &obj_file];
            if !no_std {
                ld_args.push("/usr/local/lib/libalum.a");
            }

            let ld_status = std::process::Command::new("ld").args(&ld_args).status()?;

            let _ = fs::remove_file(&asm_file);
            let _ = fs::remove_file(&obj_file);

            if !ld_status.success() {
                return Err("ld failed".into());
            }
        }
        _ => {}
    }
    Ok(())
}

fn main() {
    let cmd = Command::new("al")
        .version("0.5.2")
        .about("The Alum programming language compiler")
        .arg_required_else_help(true)
        .arg(
            Arg::new("input_files")
                .help("Input source files")
                .required(true)
                .num_args(1..),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .help("Place output in <file>")
                .value_name("file"),
        )
        .arg(
            Arg::new("preprocess")
                .short('E')
                .help("Preprocess only; do not compile, assemble or link")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("assemble")
                .short('S')
                .help("Compile only; do not assemble or link")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("compile")
                .short('c')
                .help("Compile and assemble, but do not link")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("dump_ast")
                .long("dump-ast")
                .help("Dump AST representation")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("dump_ir")
                .long("dump-ir")
                .help("Dump IR representation")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("nostdlib")
                .long("nostdlib")
                .help("Do not link with standard library")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Verbose output")
                .action(ArgAction::SetTrue),
        );

    let matches = cmd.get_matches();

    let input_files: Vec<&String> = matches.get_many("input_files").unwrap().collect();

    let input_file = input_files[0];
    let output_file = matches.get_one::<String>("output").map(|s| s.as_str());

    let verbose = matches.get_flag("verbose");
    let no_std = matches.get_flag("nostdlib");

    if verbose {
        eprintln!("Alum compiler v0.5.2");
        eprintln!("Input: {}", input_file);
        if let Some(out) = output_file {
            eprintln!("Output: {}", out);
        }
    }

    let result = if matches.get_flag("dump_ast") {
        print_ast(input_file)
    } else if matches.get_flag("dump_ir") {
        print_ir(input_file)
    } else if matches.get_flag("preprocess") {
        print_pred(input_file)
    } else if matches.get_flag("assemble") {
        compile(input_file, output_file, "asm", no_std)
    } else if matches.get_flag("compile") {
        compile(input_file, output_file, "obj", no_std)
    } else {
        compile(input_file, output_file, "bin", no_std)
    };

    if let Err(e) = result {
        eprintln!("al: error: {}", e);
        std::process::exit(1);
    }
}
