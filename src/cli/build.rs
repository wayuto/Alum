use alc::compiler::{
    CompilerError, SourceMap,
    codegen::CodeGen,
    lexer::Lexer,
    parser::Parser,
    preprocessor::Preprocessor,
    visitor::{TypeChecker, optimizer::Optimizer},
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
    cte_libs: Vec<String>,
) -> Result<String, CompilerError> {
    let src = fs::read_to_string(&input)?;

    if verbose {
        eprintln!("Preprocessing...");
    }
    let mut preprocessor = Preprocessor::new(&src, input.clone(), include_paths);
    let (processed, source_map) = preprocessor
        .preprocess()
        .map_err(|e| CompilerError::new(e, SourceMap::new()))?;

    if preprocess_only {
        println!("{}", processed);
        return Ok(String::new());
    }

    if verbose {
        eprintln!("Parsing...");
    }
    let lexer = Lexer::new(&processed);
    let mut parser = Parser::new(lexer);
    let (mut ast, parse_errors) = parser.parse_collect();

    if !parse_errors.is_empty() {
        return Err(CompilerError::report(source_map.clone(), parse_errors));
    }

    if verbose {
        eprintln!("Type checking...");
    }
    let checker = TypeChecker::new();
    let check_errors = checker.check_collect(&mut ast);

    if !check_errors.is_empty() {
        return Err(CompilerError::report(source_map.clone(), check_errors));
    }

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
    let codegen = CodeGen::new(ast, cte_libs);
    let object_code = codegen
        .generate()
        .map_err(|e| CompilerError::new(e, source_map.clone()))?;

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
    cte_libs: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let obj_file = build(
        input.clone(),
        false,
        None,
        include_paths,
        false,
        verbose,
        cte_libs.clone(),
    )?;

    let exe_file = default_exe_path(&input);
    let exe_path = std::path::Path::new(&exe_file);

    let std_lib_path = DEFAULT_STD_LIB_PATH;

    super::link::link(
        vec![obj_file.clone()],
        std_lib_path,
        exe_path.to_str().unwrap(),
        verbose,
        &cte_libs,
    )?;

    if verbose {
        eprintln!("Running: {}", exe_path.display());
    }

    let status = std::process::Command::new(&exe_path).status()?;

    let _ = std::fs::remove_file(&obj_file);
    let _ = std::fs::remove_file(&exe_path);

    if !status.success() {
        return Err(format!(
            "program exited with status: {}",
            status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "signal".to_string())
        )
        .into());
    }

    Ok(())
}

fn default_exe_path(input: &str) -> String {
    let name = std::path::Path::new(input)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(input);
    let stem = std::path::Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("a.out");
    format!("/tmp/{}", stem)
}
