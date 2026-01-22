# 🎬 Rataplay

**Rataplay** is a premium, high-performance Terminal User Interface (TUI) for searching, playing, and downloading videos. Built with Rust and inspired by [GopherTube](https://github.com/KrishnaSSH/GopherTube), it provides a sleek, modern experience for media consumption directly from your terminal.

![rataplay Demo](./assets/demo.gif)

## ✨ Features

- 🔍 **Instant Search**: Direct search from CLI or via the interactive TUI with live progress.
- 🖼️ **Visual Excellence**: High-quality thumbnails with specialized support for Kitty and WezTerm graphics protocols.
- 📺 **Versatile Playback**:
  - **External**: Play videos in an external `mpv` window.
  - **In-Terminal**: Specialized "Watch in Terminal" mode using `mpv`'s TCT output.
  - **Audio Only**: High-fidelity audio streams for background listening.
- 📥 **Background Downloads**: Multi-threaded downloads with real-time speed, progress, and ETA tracking.
- 📂 **Local Management**: Browse, play, and manage your downloaded files directly within the app.
- 🎛️ **Full Playback Control**: Play/Pause, Seek (5s/30s), and Progress tracking via IPC sockets.
- 🎹 **System Media Controls**: Native support for Play/Pause, Next/Prev, and Stop via system media keys (MPRIS/SMTC).
- ⚡ **Async Core**: Powered by Tokio for a zero-latency, non-blocking UI.
- 🎨 Toggle Themes and Animations easily with commands or from settings menu
## 🛠️ Prerequisites

Rataplay relies on the following tools:

1. **[yt-dlp](https://github.com/yt-dlp/yt-dlp)**: For metadata extraction and streaming. (Rataplay checks for updates on startup).
2. **[mpv](https://mpv.io/)**: For all playback features.

## 🚀 Installation

### From Source
```bash
cargo install --git https://github.com/mojahid8238/Rataplay.git
```
Note: Ensure that `~/.cargo/bin` is in your `PATH` to run the executable from any directory.
### From AUR (Arch Linux)
```bash
paru -S rataplay
```

## 🎮 Usage

### Launching
- **Interactive Mode**: `rataplay`
- **Direct Search**: `rataplay "lofi hip hop"`
- **Direct URL**: `rataplay https://www.youtube.com/watch?v=...`

### CLI Options
- `-v, --version`: Print version information.
- `-h, --help`: Show the custom help screen.

### Config File
- You can change the settings or your preferences from `~/.config/rataplay/config.toml`


### Keybindings

#### General & Results
| Key | Action |
|-----|--------|
| `/` or `s` | Focus Search Input |
| `j` / `k` or `arrow`| Navigate Results |
| `Enter` | Open Action Menu |
| `d` | Toggle Downloads & Local Files Panel |
| `Space` | Select for Batch Actions (Playlists) |
| `b` or `Backsp`| Go Back  |
| `q` | Quit |
|`ctrl+s`| open settings|
|`ctrl+t`| Change Themes|
|`ctrl+a`| Change Greeting screen Animation|

#### Playback Control (Active)
| Key | Action |
|-----|--------|
| `p` / `Space` | Toggle Play/Pause |
| `←` / `→` | Seek -5s / +5s |
| `[` / `]` | Seek -30s / +30s |
| `x` | Stop Playback |

#### Downloads Panel
| Key | Action |
|-----|--------|
| `j` / `k` | Navigate between Active Tasks and Local Files |
| `p` | Pause/Resume/Restart Download |
| `x` | Cancel Download / Delete Local File |
| `c` | Cleanup Garbage (.part files) |
| `d` | Delete Selected Downloads|
| `b` / `Backsp` | Go Back / Close Panel |
| `Enter` | Action Menu for Local Files |

## 🎨 Recommended Terminals
For sharp, pixel-perfect thumbnails:
- **Kitty** (Native protocol)
- **WezTerm** (Native protocol)
- **Konsole** (Sixel support)

## 📜 License
GPL 3.0 License - See `LICENSE` for details.