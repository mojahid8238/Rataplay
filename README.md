# 🎬 Vivid


**Vivid** is a premium, high-performance Terminal User Interface (TUI) for searching, playing, and downloading videos. Born from curiosity and inspired by [GopherTube](https://github.com/KrishnaSSH/GopherTube), it is built with Rust to provide a sleek, modern experience for media consumption directly from your terminal.

![Vivid Demo](./assets/demo.gif)

## ✨ Features

- 🔍 **Fast Search**: Instant video search with live progress indicators.
- 🖼️ **Visual Excellence**: High-quality thumbnails with specialized support for Kitty and WezTerm graphics protocols.
- 📺 **Versatile Playback**:
  - **External**: Play videos in an external `mpv` window.
  - **In-Terminal**: Specialized "Watch in Terminal" mode using `mpv`'s TCT output.
  - **Audio Only**: Listen to streams without video to save bandwidth.
- 📥 **Background Downloads**: Select specific formats and download videos in the background while browsing.
- 🎛️ **Full Playback Control**: Play/Pause, Seek (5s/30s), and Progress tracking directly in the TUI via IPC sockets.
- ⚡ **Async Core**: Powered by Tokio for a responsive, non-blocking UI.

## 🛠️ Prerequisites

Vivid relies on a few external tools for media handling:

1. **[yt-dlp](https://github.com/yt-dlp/yt-dlp)**: Required for metadata extraction and streaming URLs. (Vivid performs an automatic update check on startup).
2. **[mpv](https://mpv.io/)**: Required for all playback features.

## 🚀 Installation

### From Source

Ensure you have [Rust](https://www.rust-lang.org/tools/install) installed.

```bash
cargo install --git https://github.com/mojahid8238/Vivid
```
The binary should be available as `vivid` (assuming you have `~/.cargo/bin` in your `$PATH`).



## 🎮 Usage

Simply run:
```bash
vivid
```

### Keybindings

| Key | Action |
|-----|--------|
| `/` or `s` | Focus Search Input |
| `j` / `↓` | Move selection Down |
| `k` / `↑` | Move selection Up |
| `Enter` | Open Action Menu / Confirm |
| `w` | Watch (External window) |
| `t` | Watch (In Terminal) |
| `a` | Listen (Audio Only) |
| `d` | Download (Opens Format Selection) |
| `Space` / `p`| Toggle Play/Pause |
| `←` / `→` | Seek -5s / +5s |
| `[` / `]` | Seek -30s / +30s |
| `x` | Stop Playback |
| `Esc` | Cancel / Back |
| `q` | Quit Vivid |

## 🎨 Recommended Terminals

For the best visual experience (sharp thumbnails), we recommend:
- **Kitty**
- **WezTerm**
- **Konsole** (with Sixel support)

Vivid automatically detects your terminal and chooses the best possible graphics protocol.

## 📜 License

This project is licensed under the GPL 3.0 License - see the `LICENSE` file for details.
