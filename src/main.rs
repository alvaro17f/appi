mod modules;
mod utils;

use crate::{
    modules::list::list,
    utils::{
        completions::{print_completions, set_completions},
        tools::clear,
    },
};
use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use color_print::cprintln;
use modules::{delete::delete, github::github, update::update};
use std::process::exit;

#[derive(Parser, Debug, PartialEq)]
#[command(author, version, about, long_about = None)]
struct Cli {
    // If provided, generate completions for given shell
    #[arg(long = "generate", value_enum)]
    generator: Option<Shell>,
    /// List of available commands
    #[command(subcommand)]
    commands: Option<Commands>,
}

#[derive(Subcommand, Debug, PartialEq)]
enum Commands {
    /// Install an AppImage from GitHub user/repo
    #[clap(short_flag = 'i')]
    Install { repo_url: String },

    /// Update all installed AppImages
    #[clap(short_flag = 'u')]
    Update,

    /// Delete an AppImage
    #[clap(short_flag = 'd')]
    Delete,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Some(generator) = cli.generator {
        let mut cmd = Cli::command();
        if generator == Shell::Zsh || generator == Shell::Bash {
            set_completions(generator, &mut cmd);
            cprintln!("<c>{}</c> <y>completions are set", generator);
            exit(0)
        } else {
            print_completions(generator, &mut cmd);
            exit(0)
        }
    }
    match &cli.commands {
        Some(Commands::Install { repo_url }) => {
            clear()?;
            github(repo_url).await?;
        }
        Some(Commands::Update) => {
            clear()?;
            update().await?;
        }
        Some(Commands::Delete) => {
            clear()?;
            delete().await?;
        }
        None => {
            clear()?;
            list().await?;
        }
    }

    Ok(())
}
