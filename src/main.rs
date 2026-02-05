#![allow(warnings)]
use crate::{codegen::CodeGen, lexer::Lexer, parser::Parser};

mod ast;
mod codegen;
mod lexer;
mod parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src = "
    fun main(): int {
        let a: int = if true {
            2
        } else {
            0
        }
        return a
    }
    ";

    let lex = Lexer::new(src);
    let ast = Parser::new(lex).parse()?;
    println!("AST:\n{:#?}", ast.body);

    let codegen = CodeGen::new(ast);
    let object_code = codegen.generate()?;

    println!("\nCompiled successfully!");
    println!("Object code size: {} bytes", object_code.len());

    println!(
        "\nFirst {} bytes of object code:",
        object_code.len().min(128)
    );
    for (i, chunk) in object_code.chunks(16).enumerate() {
        print!("{:04x}: ", i * 16);
        for byte in chunk {
            print!("{:02x} ", byte);
        }
        println!();
        if i * 16 >= 128 {
            break;
        }
    }

    let obj_path = "output.o";
    std::fs::write(obj_path, object_code)?;
    println!("\nObject code written to: {}", obj_path);

    let exe_path = "output";
    println!("Linking object file to executable...");

    let output = std::process::Command::new("cc")
        .arg(obj_path)
        .arg("-o")
        .arg(exe_path)
        .output()?;

    if !output.status.success() {
        eprintln!("Linker failed:");
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        return Err("Linking failed".into());
    }

    println!("Executable created: {}", exe_path);

    println!("\nRunning the executable:");
    let run_output = std::process::Command::new(format!("./{}", exe_path))
        .current_dir(".")
        .output()?;

    println!("Exit code: {}", run_output.status.code().unwrap_or(-1));
    if !run_output.stdout.is_empty() {
        println!("stdout: {}", String::from_utf8_lossy(&run_output.stdout));
    }
    if !run_output.stderr.is_empty() {
        println!("stderr: {}", String::from_utf8_lossy(&run_output.stderr));
    }

    println!("\nYou can run the executable with: ./{}", exe_path);

    Ok(())
}
