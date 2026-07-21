use crate::compiler::{
    CompilerError, SourceMap, codegen::CodeGen, lexer::Lexer, parser::Parser,
    preprocessor::Preprocessor, visitor::checker::TypeChecker, visitor::optimizer::Optimizer,
};
use std::fs;

pub const DEFAULT_STD_LIB_PATH: &str = "/usr/local/lib/libalum.a";

pub fn build(
    input: String,
    print_ast: bool,
    object: Option<String>,
    include_paths: Vec<String>,
    preprocess_only: bool,
    verbose: bool,
) -> Result<String, CompilerError> {
    let src = fs::read_to_string(&input)?;

    if verbose {
        eprintln!("Preprocessing...");
    }
    let mut preprocessor = Preprocessor::new(&src, input.clone(), include_paths);
    let (processed, source_map) = preprocessor
        .preprocess()
        .map_err(|e| CompilerError::new(e, src.clone(), input.clone(), SourceMap::new()))?;

    if preprocess_only {
        println!("{}", processed);
        return Ok(String::new());
    }

    if verbose {
        eprintln!("Parsing...");
    }
    let lexer = Lexer::new(&processed);
    let mut parser = Parser::new(lexer);
    let mut ast = parser
        .parse()
        .map_err(|e| CompilerError::new(e, processed.clone(), input.clone(), source_map.clone()))?;

    if verbose {
        eprintln!("Type checking...");
    }
    let checker = TypeChecker::new();
    checker
        .check(&mut ast)
        .map_err(|e| CompilerError::new(e, processed.clone(), input.clone(), source_map.clone()))?;

    if print_ast {
        println!("{}", ast);
        return Ok(String::new());
    }

    if verbose {
        eprintln!("Optimizing...");
    }
    let optimizer = Optimizer::new();
    optimizer.optimize(&mut ast);

    if verbose {
        eprintln!("Generating code...");
    }
    let codegen = CodeGen::new(ast);
    let object_code = codegen
        .generate()
        .map_err(|e| CompilerError::new(e, processed.clone(), input.clone(), source_map.clone()))?;

    let output_file = if let Some(obj_file) = object {
        obj_file
    } else {
        let input_path = std::path::Path::new(&input);
        let stem = input_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        format!("/tmp/{}.o", stem)
    };

    if verbose {
        eprintln!("Writing object file to: {}", output_file);
    }
    fs::write(&output_file, object_code)?;

    Ok(output_file)
}

pub fn exec_run(
    input: String,
    include_paths: Vec<String>,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let obj_file = build(input.clone(), false, None, include_paths, false, verbose)?;

    let exe_file = if input.ends_with(".al") {
        input.replace(".al", "")
    } else {
        input.clone()
    };

    let exe_path = std::path::Path::new(&exe_file);

    let std_lib_path = DEFAULT_STD_LIB_PATH;

    super::link::link(
        vec![obj_file],
        std_lib_path,
        exe_path.to_str().unwrap(),
        verbose,
    )?;

    let exe_file_abs = exe_path.canonicalize()?;

    if verbose {
        eprintln!("Running: {}", exe_file_abs.display());
    }

    std::process::Command::new(&exe_file_abs).status()?;

    std::fs::remove_file(&exe_file_abs)?;

    Ok(())
}
