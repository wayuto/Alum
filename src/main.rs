use crate::{
    compiler::Compiler, gvm::Gvm, lexer::Lexer, parser::Parser, preprocessor::Preprocessor,
};
use clap::{Arg, Command};
use std::fs;

pub mod ast;
pub mod bytecode;
pub mod compiler;
pub mod gvm;
pub mod lexer;
pub mod parser;
pub mod preprocessor;
pub mod token;

fn main() {
    let matches = Command::new("gos")
        .version("0.2.7#rust")
        .about("Gos interpreter implemented in Rust")
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
                .help("Compile the Gos source file"),
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
        )
        .get_matches();

    if let Some(file) = matches.get_one::<String>("FILE") {
        let src = fs::read_to_string(file).unwrap();
        let mut preprocessor = Preprocessor::new(src.as_str());
        let code = preprocessor.preprocess();
        let lexer = Lexer::new(code.as_str());
        let mut parser = Parser::new(lexer);
        let ast = parser.parse();
        let mut compiler = Compiler::new();
        let bytecode = compiler.compile(ast);
        let mut gvm = Gvm::new(bytecode);
        gvm.run();
    } else if let Some(file) = matches.get_one::<String>("ast") {
        let src = fs::read_to_string(file).unwrap();
        let mut preprocessor = Preprocessor::new(src.as_str());
        let code = preprocessor.preprocess();
        let lexer = Lexer::new(code.as_str());
        let mut parser = Parser::new(lexer);
        let ast = parser.parse();
        println!("{:#?}", ast);
    } else if let Some(file) = matches.get_one::<String>("preprocess") {
        let src = fs::read_to_string(file).unwrap();
        let mut preprocessor = Preprocessor::new(src.as_str());
        let code = preprocessor.preprocess();
        println!("{}", code);
    } else if let Some(file) = matches.get_one::<String>("compile") {
        println!("No implemented yet")
    } else if let Some(file) = matches.get_one::<String>("disassemble") {
        let src = fs::read_to_string(file).unwrap();
        let mut preprocessor = Preprocessor::new(src.as_str());
        let code = preprocessor.preprocess();
        let lexer = Lexer::new(code.as_str());
        let mut parser = Parser::new(lexer);
        let ast = parser.parse();
        let mut compiler = Compiler::new();
        let bytecode = compiler.compile(ast);
        bytecode.print();
    }
}
