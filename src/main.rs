#![allow(warnings)]
use crate::{
    bytecode::GVM, lexer::Lexer, native::IRGen, parser::Parser, preprocessor::Preprocessor,
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
    println!("{:?}", ir);
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
        // cmd.clone().print_help().unwrap();
        // std::process::exit(0);
        let file = "/home/w/Gos/foo.gos";
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
        println!("{:#?}", ast);
        let mut irgen = IRGen::new();
        let ir = irgen.compile(ast);
        println!("{:#?}", ir);
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
