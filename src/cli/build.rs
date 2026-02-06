use crate::compiler::{
    codegen::CodeGen, lexer::Lexer, parser::Parser as AstParser, preprocessor::Preprocessor,
};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn build(
    input: String,
    output: Option<String>,
    compile_only: bool,
    print_ast: bool,
    object: Option<String>,
    include_paths: Vec<String>,
    preprocess_only: bool,
    verbose: bool,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let input_path = PathBuf::from(&input);
    let src = fs::read_to_string(&input_path)?;

    let mut preprocessor = Preprocessor::new(
        &src,
        input_path.to_string_lossy().to_string(),
        include_paths,
    );
    let processed_src = preprocessor.preprocess()?;

    if preprocess_only {
        print!("{}", processed_src);
        return Ok(None);
    }

    if verbose {
        eprintln!("Processing: {}", input);
    }

    let lex = Lexer::new(&processed_src);
    let ast = AstParser::new(lex).parse()?;

    if print_ast {
        println!("{:#?}", ast.body);
    }

    if verbose {
        eprintln!("Generating code...");
    }

    let codegen = CodeGen::new(ast);
    let object_code = codegen.generate()?;

    let input_stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let exe_name = output.as_deref().unwrap_or(input_stem);

    let obj_path = object.unwrap_or_else(|| format!("{}.o", exe_name));

    fs::write(&obj_path, object_code)?;

    if verbose {
        eprintln!("Object file written to: {}", obj_path);
    }

    if compile_only {
        return Ok(Some(obj_path));
    }

    Ok(Some(obj_path))
}

pub fn exec_run(
    input: String,
    include_paths: Vec<String>,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let obj_path = build(input.clone(), None, false, false, None, include_paths, false, verbose)?
        .ok_or("No object file generated")?;

    let std_lib_path = std::env::var("ALUM_STD_PATH").unwrap_or_else(|_| {
        let project_root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        format!("{}/alum-std/target/release/libalum_std.a", project_root)
    });

    let input_path = PathBuf::from(&input);
    let input_stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let exe_path = input_stem.to_string();
    
    if verbose {
        eprintln!("Linking with standard library: {}", std_lib_path);
    }
    
    crate::cli::link::link_objects(vec![obj_path.clone()], &std_lib_path, &exe_path)?;

    fs::remove_file(&obj_path)?;

    if verbose {
        eprintln!("Running: {}", exe_path);
    }

    let run_output = Command::new(&input_stem).current_dir(".").output()?;

    if !run_output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&run_output.stdout));
    }
    if !run_output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&run_output.stderr));
    }

    Ok(())
}
