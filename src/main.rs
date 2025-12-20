#![allow(warnings)]
use crate::{
    bytecode::GVM,
    lexer::Lexer,
    native::{CodeGen, IRGen},
    parser::Parser,
    preprocessor::Preprocessor,
};
use clap::{Arg, ArgAction, Command};
use std::{fs, path::Path};

pub mod ast;
pub mod bytecode;
pub mod error;
pub mod lexer;
pub mod native;
pub mod parser;
pub mod preprocessor;
pub mod token;

fn run_bytecode(file: &String) -> () {
    let src = fs::read_to_string(file).unwrap();
    let path = Path::new(&file.clone())
        .parent()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let mut preprocessor = Preprocessor::new(src.as_str(), path);
    let code = preprocessor.preprocess();
    let lexer = Lexer::new(code.as_str());
    let mut parser = Parser::new(lexer);
    let ast = parser.parse();
    let mut compiler = bytecode::Compiler::new();
    let bytecode = compiler.compile(ast);
    let mut gvm = GVM::new(bytecode);
    gvm.run();
}

fn print_ast(file: &String) -> () {
    let src = fs::read_to_string(file).unwrap();
    let path = Path::new(&file.clone())
        .parent()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let mut preprocessor = Preprocessor::new(src.as_str(), path);
    let code = preprocessor.preprocess();
    let lexer = Lexer::new(code.as_str());
    let mut parser = Parser::new(lexer);
    let ast = parser.parse();
    println!("{:#?}", ast);
}

fn print_pred(file: &String) -> () {
    let src = fs::read_to_string(file).unwrap();
    let path = Path::new(&file.clone())
        .parent()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let mut preprocessor = Preprocessor::new(src.as_str(), path);
    let code = preprocessor.preprocess();
    println!("{}", code);
}

fn print_bytecode(file: &String) -> () {
    let src = fs::read_to_string(file).unwrap();
    let path = Path::new(&file.clone())
        .parent()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let mut preprocessor = Preprocessor::new(src.as_str(), path);
    let code = preprocessor.preprocess();
    let lexer = Lexer::new(code.as_str());
    let mut parser = Parser::new(lexer);
    let ast = parser.parse();
    let mut compiler = bytecode::Compiler::new();
    let bytecode = compiler.compile(ast);
    bytecode.print();
}

fn compile_native(file: &String, typ: &str, no_std: bool) -> () {
    let src = fs::read_to_string(file).unwrap();
    let path = Path::new(&file)
        .parent()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let mut preprocessor = Preprocessor::new(&src, path);
    let code = preprocessor.preprocess();
    let lexer = Lexer::new(&code);
    let mut parser = Parser::new(lexer);
    let ast = parser.parse();
    let mut irgen = IRGen::new();
    let ir = irgen.compile(ast);
    let mut codegen = CodeGen::new(ir);
    let assembly = codegen.compile();

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
            fs::write(&asm_file, &assembly).unwrap();
        }
        "obj" => {
            fs::write(&asm_file, &assembly).unwrap();
            let nasm_status = std::process::Command::new("nasm")
                .args(&["-f", "elf64", "-o", &obj_file, &asm_file])
                .status()
                .expect("Failed to run nasm");
            if !nasm_status.success() {
                let _ = fs::remove_file(&asm_file);
                println!("nasm failed");
                std::process::exit(1);
            }
            let _ = fs::remove_file(&asm_file);
        }
        "bin" => {
            fs::write(&asm_file, &assembly).unwrap();
            let nasm_status = std::process::Command::new("nasm")
                .args(&["-f", "elf64", "-o", &obj_file, &asm_file])
                .status()
                .expect("Failed to run nasm");
            if !nasm_status.success() {
                println!("nasm failed");
                std::process::exit(1);
            }

            let mut ld_args = vec!["-o", &bin_file, &obj_file];
            if !no_std {
                ld_args.push("/usr/local/lib/libgos.a");
            }
            let ld_status = std::process::Command::new("ld")
                .args(&ld_args)
                .status()
                .expect("Failed to run ld");
            if !ld_status.success() {
                let _ = fs::remove_file(&asm_file);
                let _ = fs::remove_file(&obj_file);
                println!("ld failed");
                std::process::exit(1);
            }

            let _ = fs::remove_file(&asm_file);
            let _ = fs::remove_file(&obj_file);
        }
        _ => {}
    }
}

fn main() {
    let cmd = Command::new("gos")
        .version("0.4.0")
        .about("The Gos programming language")
        .arg(Arg::new("FILE").help("Run the Gos source file"))
        .arg(
            Arg::new("ast")
                .short('a')
                .long("ast")
                .help("Print AST of the Gos source file"),
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
        )
        .arg(
            Arg::new("disassemble")
                .short('d')
                .long("disassemble")
                .help("Run the Gos source file"),
        );

    if std::env::args().len() == 1 {
        cmd.clone().print_help().unwrap();
        std::process::exit(0);
    }

    let matches = cmd.get_matches();

    if let Some(file) = matches.get_one::<String>("FILE") {
        run_bytecode(file);
    } else if let Some(file) = matches.get_one::<String>("ast") {
        print_ast(file);
    } else if let Some(file) = matches.get_one::<String>("preprocess") {
        print_pred(file);
    } else if let Some(file) = matches.get_one::<String>("disassemble") {
        print_bytecode(file);
    } else if let Some(file) = matches.get_one::<String>("compile") {
        if matches.get_flag("assembly") {
            compile_native(file, "asm", false);
        } else if matches.get_flag("object") {
            compile_native(file, "obj", false);
        } else {
            compile_native(file, "bin", matches.get_flag("nostd"));
        }
    }
}
