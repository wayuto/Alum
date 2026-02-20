use clap::Parser;

#[derive(Parser)]
#[command(name = "alc")]
#[command(version = "0.9.1")]
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

    #[arg(long, help = "Output AST representation")]
    pub ast: bool,

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
}
