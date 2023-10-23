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
use modules::{
    aur_download::aur_download,
    aur_search::{aur_search, get_appimage_url},
    delete::delete,
    github_download::github_download,
    github_search::github_search,
    update::update,
};
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
    /// Search & install an AppImage
    #[clap(short_flag = 's')]
    Search {
        args: Option<String>,
        #[arg(short = 'g', long = "github")]
        github: bool,
    },

    /// Install an AppImage
    #[clap(short_flag = 'i')]
    Install {
        args: Option<String>,
        #[arg(short = 'g', long = "github")]
        github: bool,
    },
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
        Some(Commands::Search { args, github }) => {
            clear()?;
            if *github {
                github_search(args.as_ref().unwrap()).await?;
                exit(0)
            } else if args.is_some() {
                aur_search(args.as_ref().unwrap()).await?;
                exit(0)
            } else {
                cprintln!("<r>Missing arguments</r>");
                exit(1)
            }
        }
        Some(Commands::Install { args, github }) => {
            clear()?;
            if *github {
                github_download(args.as_ref().unwrap()).await?;
                exit(0)
            } else if args.is_some() {
                let name = &args.as_ref().unwrap();
                let appimage_url = get_appimage_url(name).await?;
                aur_download(&appimage_url, name).await?;
                exit(0)
            } else {
                cprintln!("<r>Missing arguments</r>");
                exit(1)
            }
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
