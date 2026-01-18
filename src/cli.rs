use clap::Parser;

#[derive(Parser)]
#[command(name = "Rataplay")]
#[command(author = "Mojahid")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(disable_version_flag = true)]
#[command(help_template = "NAME:
   {name} - Terminal YouTube Search & Play

USAGE:
   rataplay [query] [global options]

VERSION:
   {version}

DESCRIPTION:
   {name} is a terminal-based YouTube search and video player application.
   Navigate through YouTube content efficiently using keyboard shortcuts and enjoy
   seamless video playback directly from your terminal.

   Controls:
     • Type your search query and press Enter
     • Use ↑/↓ to navigate results
     • Press s to search
     • Press q to quit
     • Press Esc to go back or exit

AUTHOR:
   {author}

GLOBAL OPTIONS:
{options}
")]
pub struct Cli {
    /// Search query to run on startup
    pub query: Option<String>,

    /// print the version
    #[arg(short = 'v', long = "version", action = clap::ArgAction::Version)]
    pub show_version: Option<bool>,
}
