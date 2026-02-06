use clap::{Parser, Subcommand};

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Create a new project")]
    New {
        name: String,
        #[arg(short, long, default_value = "alum")]
        language: String,
    },
    #[command(about = "Run the project")]
    Run,
    #[command(about = "Build the project")]
    Build,
    #[command(about = "Clean the project")]
    Clean,
    #[command(about = "Add a dependency")]
    Add {
        name: String,
        #[arg(short, long)]
        url: Option<String>,
        #[arg(short, long)]
        local: Option<String>,
        #[arg(short, long)]
        git: bool,
        #[arg(short, long)]
        tag: Option<String>,
    },
    #[command(name = "rm", about = "Remove a dependency")]
    Remove { name: String },
    #[command(about = "Pull the dependencies")]
    Sync,
}

#[derive(Parser)]
pub struct Arg {
    #[command(subcommand)]
    pub cmd: Commands,
}
