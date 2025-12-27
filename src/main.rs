#![allow(warnings)]
use crate::codegen::CodeGen;
use crate::irgen::IRGen;
use crate::{lexer::Lexer, parser::Parser, preprocessor::Preprocessor};
use clap::{Arg, ArgAction, Command};
use std::{fs, path::Path};

pub mod ast;
pub mod codegen;
pub mod gir;
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

fn compile_native(file: &String, typ: &str, no_std: bool) -> Result<(), Box<dyn std::error::Error>> {
    let src = fs::read_to_string(file)?;
    let path = Path::new(&file)
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

    let stem = if let Some(idx) = file.rfind('.') {
        &file[..idx]
    } else {
        file.as_str()
    };
    let asm_file = format!("{}.s", stem);
    let obj_file = format!("{}.o", stem);
    let bin_file = stem.to_string();

    match typ {
        "asm" => {
            fs::write(&asm_file, &assembly)?;
        }
        "obj" => {
            fs::write(&asm_file, &assembly)?;
            let nasm_status = std::process::Command::new("nasm")
                .args(&["-f", "elf64", "-o", &obj_file, &asm_file])
                .status()?;
            if !nasm_status.success() {
                let _ = fs::remove_file(&asm_file);
                return Err("nasm failed".into());
            }
            let _ = fs::remove_file(&asm_file);
        }
        "bin" => {
            fs::write(&asm_file, &assembly)?;
            let nasm_status = std::process::Command::new("nasm")
                .args(&["-f", "elf64", "-o", &obj_file, &asm_file])
                .status()?;
            if !nasm_status.success() {
                return Err("nasm failed".into());
            }

            let mut ld_args = vec!["-o", &bin_file, &obj_file];
            if !no_std {
                ld_args.push("/usr/local/lib/libgos.a");
            }
            let ld_status = std::process::Command::new("ld")
                .args(&ld_args)
                .status()?;
            if !ld_status.success() {
                let _ = fs::remove_file(&asm_file);
                let _ = fs::remove_file(&obj_file);
                return Err("ld failed".into());
            }

            let _ = fs::remove_file(&asm_file);
            let _ = fs::remove_file(&obj_file);
        }
        _ => {}
    }
    Ok(())
}

fn main() {
    let cmd = Command::new("gos")
        .version("0.5.2")
        .about("The Gos programming language")
        .arg(
            Arg::new("ast")
                .short('a')
                .long("ast")
                .help("Print AST of the Gos source file"),
        )
        .arg(
            Arg::new("ir")
                .short('i')
                .long("ir")
                .help("Print IR of the Gos source file"),
        )
        .arg(
            Arg::new("compile")
                .short('c')
                .long("compile")
                .help("Compile the Gos source file to native"),
        )
        .arg(
            Arg::new("assembly")
                .short('s')
                .help("Compile the Gos source file to assembly")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("object")
                .short('o')
                .help("Compile the Gos source file to object")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("nostd")
                .short('n')
                .help("Do not link the Gos Standard Library")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("preprocess")
                .short('p')
                .long("preprocess")
                .help("Print the preprocessed Gos source file"),
        );

    if std::env::args().len() == 1 {
        if let Err(e) = cmd.clone().print_help() {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        std::process::exit(0);
    }

    let matches = cmd.get_matches();
    let result = if let Some(file) = matches.get_one::<String>("ast") {
        print_ast(file)
    } else if let Some(file) = matches.get_one::<String>("ir") {
        print_ir(file)
    } else if let Some(file) = matches.get_one::<String>("preprocess") {
        print_pred(file)
    } else if let Some(file) = matches.get_one::<String>("compile") {
        if matches.get_flag("assembly") {
            compile_native(file, "asm", false)
        } else if matches.get_flag("object") {
            compile_native(file, "obj", false)
        } else {
            compile_native(file, "bin", matches.get_flag("nostd"))
        }
    } else {
        Ok(())
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
