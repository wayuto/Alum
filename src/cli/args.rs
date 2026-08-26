use clap::Parser;

#[derive(Parser)]
#[command(name = "alc")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Alum compiler")]
pub struct Cli {
    #[arg(
        value_name = "INPUT",
        help = "Input files (.al source files or .o/.obj object files)"
    )]
    pub input: Vec<String>,

    #[arg(short = 'o', long, value_name = "FILE", help = "Output file name")]
    pub output: Option<String>,

    #[arg(short = 'c', long, help = "Compile only, do not link")]
    pub compile_only: bool,

    #[arg(long = "emit-ast", help = "Output AST representation")]
    pub emit_ast: bool,

    #[arg(long, help = "Dump optimized IR to stderr, then continue compiling")]
    pub emit_ir: bool,

    #[arg(
        long,
        help = "Dump generated assembly to stderr, then continue compiling"
    )]
    pub emit_asm: bool,

    #[arg(short = 'r', long, help = "Compile and run immediately")]
    pub run: bool,

    #[arg(short = 'I', value_name = "DIR", action = clap::ArgAction::Append, help = "Add include directory")]
    pub include_paths: Vec<String>,

    #[arg(
        short = 'E',
        help = "Preprocess only; do not compile, assemble or link"
    )]
    pub preprocess_only: bool,

    #[arg(long, help = "Do not link with standard library")]
    pub nostdlib: bool,

    #[arg(short = 'v', long, help = "Verbose output")]
    pub verbose: bool,

    #[arg(long, value_name = "TYPE", help = "Build library (static or shared)")]
    pub library: Option<String>,

    #[arg(
        long,
        value_name = "PATH",
        action = clap::ArgAction::Append,
        help = "Shared library to dlopen for compile-time evaluation of fun(extern, pure) functions"
    )]
    pub cte_lib: Vec<String>,
}
