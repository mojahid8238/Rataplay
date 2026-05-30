use clap::Parser;

#[derive(Parser)]
#[command(name = "Rataplay")]
#[command(author = "Mojahid")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(disable_version_flag = true)]
#[command(help_template = "\
NAME:
   {name} - Terminal YouTube Search & Play

USAGE:
   rataplay [query] [options]

VERSION:
   {version}

DESCRIPTION:
   {name} is a terminal-based YouTube search and video player application.
   Type a search query or paste a YouTube URL to get started.

   Navigation:
     s, /              Focus search bar
     j, Down           Move selection down
     k, Up             Move selection up
     Enter             Open action menu / Load more results
     Space             Toggle multi-select
     Backspace, b      Go back / Close panel
     Tab               Switch to downloads panel
     Esc               Go back / Exit editing mode
     q                 Quit

   Playback:
     p                 Pause / Resume
     x                 Stop playback
     Left              Seek backward 5s
     Right             Seek forward 5s / Play selected externally
     [                 Seek backward 30s
     ]                 Seek forward 30s

   Actions (from action menu):
     w                 Watch externally (mpv window)
     t                 Watch in terminal (mpv tct)
     a                 Listen (audio only)
     d                 Download
     o                 Open in browser
     c                 Copy URL / Channel ID to clipboard

   Controls:
     Ctrl+s            Toggle Settings menu
     Ctrl+t            Cycle theme
     Ctrl+a            Cycle logo animation
     Ctrl+l            Toggle live stream filter
     Ctrl+p            Toggle playlist filter
     Ctrl+u            In editing: clear to line start
     Ctrl+k            In editing: clear to line end
     Ctrl+w            In editing: delete word backwards
     Ctrl+a            In editing: move to line start
     Ctrl+e            In editing: move to line end
     Ctrl+Left         In editing: move left by word
     Ctrl+Right        In editing: move right by word

   Search:
     Enter             In search bar: Execute search
     Ctrl+u/k/w/a/e    Editing shortcuts (see Controls)

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
