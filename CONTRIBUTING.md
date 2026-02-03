# Contributing to Rataplay

Thank you for your interest in contributing to Rataplay! We welcome all contributions, from bug reports and feature requests to code changes and documentation improvements.

## 🚀 Getting Started

### Prerequisites

To build and run Rataplay from source, you will need:

1.  **Rust Toolchain**: Install via [rustup.rs](https://rustup.rs/).
2.  **System Dependencies** (Linux):
    ```bash
    sudo apt-get update
    sudo apt-get install -y libdbus-1-dev libasound2-dev pkg-config
    ```
3.  **Runtime Dependencies**:
    - [yt-dlp](https://github.com/yt-dlp/yt-dlp)
    - [mpv](https://mpv.io/)

### Building from Source

```bash
git clone https://github.com/mojahid8238/Rataplay.git
cd Rataplay
cargo build
```

## 🛠️ How to Contribute

### Reporting Bugs

- Check the [Issues](https://github.com/mojahid8238/Rataplay/issues) to see if the bug has already been reported.
- Use a clear and descriptive title.
- Describe the steps to reproduce the issue.
- Include information about your OS and terminal emulator.

### Suggesting Enhancements

- Open a new issue with the "feature request" tag.
- Explain why the feature would be useful and how it should work.

### Pull Requests

1.  **Fork** the repository.
2.  **Create a branch** for your change: `git checkout -b feature/your-feature-name` or `git checkout -b fix/your-fix-name`.
3.  **Make your changes**. Ensure your code follows the existing style.
4.  **Run tests and formatting**:
    ```bash
    cargo fmt --all
    cargo test
    ```
5.  **Commit your changes** with descriptive commit messages.
6.  **Push to your fork** and **open a Pull Request** against the `main` branch.

## 🎨 Coding Guidelines

- Follow standard Rust naming conventions.
- Use `cargo fmt` to format your code before committing.
- Keep functions and modules focused and concise.
- Add comments explaining complex logic where necessary.

## 💬 Community

If you have questions or want to discuss ideas, feel free to open a [Discussion](https://github.com/mojahid8238/Rataplay/discussions) or join our community (if applicable).

---

By contributing to Rataplay, you agree that your contributions will be licensed under the [GPL 3.0 License](./LICENSE).
