use bincode::{
    config,
    serde::{decode_from_slice, encode_to_vec},
};

use crate::{
    bytecode::{Bytecode, Compiler},
    lexer::Lexer,
    parser::Parser,
    preprocessor::Preprocessor,
};
use std::{fs, path::Path};

pub fn compile(source: String) -> () {
    let output = if let Some(idx) = source.rfind('.') {
        format!("{}.gbc", &source.clone()[..idx])
    } else {
        format!("{}.gbc", source.clone())
    };

    let src = fs::read_to_string(source.clone()).unwrap();
    let path = Path::new(&source.clone())
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
    let mut compiler = Compiler::new();
    let bytecode = compiler.compile(ast);

    let encoded: Vec<u8> = encode_to_vec(&bytecode, bincode::config::standard()).unwrap();
    match std::fs::write(&output, encoded.clone()) {
        Ok(_) => {}
        Err(e) => {
            println!("{}", e)
        }
    }
}

pub fn load(source: String) -> Bytecode {
    let bytes = fs::read(source).expect("Failed to read file");
    let (bytecodes, _): (Bytecode, _) =
        decode_from_slice(&bytes, config::standard()).expect("Failed to read bytes");
    bytecodes
}
