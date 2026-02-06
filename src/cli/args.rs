use clap::Parser;

#[derive(Parser)]
#[command(name = "alc")]
#[command(version = "0.7.0")]
#[command(about = "Alum compiler")]
pub struct Cli {
    #[arg(value_name = "INPUT")]
    pub input: Vec<String>,

    #[arg(short = 'o', long)]
    pub output: Option<String>,

    #[arg(short = 'c', long)]
    pub compile_only: bool,

    #[arg(long)]
    pub ast: bool,

    #[arg(short = 'r', long)]
    pub run: bool,

    #[arg(short = 'I', value_name = "DIR", action = clap::ArgAction::Append)]
    pub include_paths: Vec<String>,

    #[arg(short = 'E')]
    pub preprocess_only: bool,

    #[arg(long)]
    pub nostdlib: bool,

    #[arg(short = 'v', long)]
    pub verbose: bool,
}
