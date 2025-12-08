use crate::{
    bytecode::GVM,
    lexer::Lexer,
    parser::Parser,
    preprocessor::Preprocessor,
    serialize::{compile, load},
};
use clap::{Arg, ArgAction, Command};
use std::{fs, path::Path};

pub mod ast;
pub mod bytecode;
pub mod lexer;
pub mod native;
pub mod parser;
pub mod preprocessor;
pub mod serialize;
pub mod token;

fn run_bytecode(file: &String) -> () {
    let ext = Path::new(file)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase());
    match ext.as_deref() {
        Some("gbc") => {
            let bytecode = load(file.to_string());
            let mut gvm = GVM::new(bytecode);
            gvm.run();
        }
        _ => {
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
    }
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
    let ext = Path::new(file)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase());
    match ext.as_deref() {
        Some("gbc") => {
            load(file.to_string()).print();
        }
        _ => {
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
    }
}

fn compile_native(file: &String, typ: &str, is_link: bool) -> () {
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
    let mut compiler = native::Compiler::new();
    let assembly = compiler.compile(ast);

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

            let lib_path = format!("/tmp/lib.o");
            let mut ld_args = vec!["-o", &bin_file, &lib_path, &obj_file];
            if is_link {
                ld_args.push("-lc");
                ld_args.push("-I/lib64/ld-linux-x86-64.so.2");
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
        .version("0.3.0")
        .about("The Gos programming language")
        .arg(Arg::new("FILE").help("Run the Gos source/bytecode file"))
        .arg(
            Arg::new("ast")
                .short('a')
                .long("ast")
                .help("Print AST of the Gos source file"),
        )
        .arg(
            Arg::new("bytecode")
                .short('b')
                .long("bytecode")
                .help("Compile the Gos source file to bytecode"),
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
            Arg::new("libc")
                .short('l')
                .help("Link the libc when compiling to native")
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
                .help("Run the Gos source/bytecode file"),
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
    } else if let Some(file) = matches.get_one::<String>("bytecode") {
        compile(file.to_string());
    } else if let Some(file) = matches.get_one::<String>("disassemble") {
        print_bytecode(file);
    } else if let Some(file) = matches.get_one::<String>("compile") {
        if matches.get_flag("assembly") {
            compile_native(file, "asm", false);
        } else if matches.get_flag("object") {
            compile_native(file, "obj", false);
        } else {
            let is_link = matches.get_flag("libc");
            compile_native(file, "bin", is_link);
        }
    }
}
