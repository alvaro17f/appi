use anyhow::Result;
use appi::{
    api::{aur::AUR, github::GITHUB},
    modules::{delete::delete, list::list, update::update},
    utils::{completions::Completions, tools::Tools},
};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use color_print::cprintln;
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
    Tools.clear()?;
    cprintln!("<g,s>##################################");
    cprintln!("<g,s>~> APPI</> - <y>AppImage Installer</>");
    cprintln!("<g,s>##################################");
    let cli = Cli::parse();
    if let Some(generator) = cli.generator {
        let mut cmd = Cli::command();
        if generator == Shell::Zsh || generator == Shell::Bash {
            Completions::set_completions(generator, &mut cmd);
            cprintln!("<c>{}</c> <y>completions are set", generator);
            exit(0)
        } else {
            Completions::print_completions(generator, &mut cmd);
            exit(0)
        }
    }
    match &cli.commands {
        Some(Commands::Search { args, github }) => {
            if *github {
                GITHUB::search(args.as_ref().unwrap()).await?;
                exit(0)
            } else if args.is_some() {
                AUR::search(args.as_ref().unwrap()).await?;
                exit(0)
            } else {
                cprintln!("<r>Missing arguments</r>");
                exit(1)
            }
        }
        Some(Commands::Install { args, github }) => {
            if *github {
                GITHUB::download(args.as_ref().unwrap()).await?;
                exit(0)
            } else if args.is_some() {
                let name = &args.as_ref().unwrap();
                AUR::download(name).await?;
                exit(0)
            } else {
                cprintln!("<r>Missing arguments</r>");
                exit(1)
            }
        }
        Some(Commands::Update) => {
            update().await?;
        }
        Some(Commands::Delete) => {
            delete().await?;
        }
        None => {
            list().await?;
        }
    }
    println!();
    Ok(())
}
