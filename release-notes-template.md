# __NAME__ __TAG__

__DESCRIPTION__

## 🚀 Installation

### ⚡ Quick Install (Recommended)
Install pre-built binaries with [cargo-binstall](https://github.com/cargo-bins/cargo-binstall):

```bash
cargo binstall __NAME__
```

> **Why cargo-binstall?** Downloads pre-compiled binaries directly from GitHub releases - no compilation time, no build dependencies required!

### 🔨 Install from Source
Compile from source via [crates.io](https://crates.io/crates/__NAME__):

```bash
cargo install __NAME__
```

### 📦 Manual Binary Download
Download pre-built binaries for your platform:

| Platform | Download | Compatibility |
|----------|----------|---------------|
| **Linux x86_64** | [__NAME__-__TAG__-x86_64-unknown-linux-gnu.tar.gz](https://github.com/toolprint/vibe-workspace/releases/download/__TAG__/__NAME__-__TAG__-x86_64-unknown-linux-gnu.tar.gz) | Most Linux distributions |
| **macOS Universal** | [__NAME__-__TAG__-universal2-apple-darwin.tar.gz](https://github.com/toolprint/vibe-workspace/releases/download/__TAG__/__NAME__-__TAG__-universal2-apple-darwin.tar.gz) | **Recommended** - Works on Intel & Apple Silicon |
| **macOS Intel** | [__NAME__-__TAG__-x86_64-apple-darwin.tar.gz](https://github.com/toolprint/vibe-workspace/releases/download/__TAG__/__NAME__-__TAG__-x86_64-apple-darwin.tar.gz) | Intel-based Macs only |
| **macOS Apple Silicon** | [__NAME__-__TAG__-aarch64-apple-darwin.tar.gz](https://github.com/toolprint/vibe-workspace/releases/download/__TAG__/__NAME__-__TAG__-aarch64-apple-darwin.tar.gz) | M1/M2/M3/M4 Macs only |

**Installation steps:**
1. Download the appropriate archive for your platform
2. Extract: `tar -xzf __NAME__-__TAG__-<platform>.tar.gz`
3. Move binary to PATH: `sudo mkdir -p /usr/local/bin && sudo mv vibe /usr/local/bin/`
4. Verify installation: `vibe --version`

### 🔐 Verify Downloads
All release artifacts include SHA256 checksums for security verification:

```bash
# Download checksums file
wget https://github.com/toolprint/vibe-workspace/releases/download/__TAG__/SHA256SUMS

# Verify your download
shasum -a 256 -c SHA256SUMS --ignore-missing
```

## 📋 What's New

See [CHANGELOG.md](https://github.com/toolprint/vibe-workspace/blob/main/CHANGELOG.md) for detailed changes in this release.

## 🆘 Troubleshooting

### Installation Issues
- **cargo-binstall fails**: Try `cargo install __NAME__` to compile from source
- **Permission denied**: Use `sudo` for system-wide installation or install to `~/.local/bin/`
- **Command not found**: Ensure `~/.cargo/bin` or `/usr/local/bin` is in your PATH

### Platform Support
- **Windows**: Not currently supported (Linux and macOS only)
- **Other architectures**: Use `cargo install __NAME__` to compile for your platform

### Getting Help
- 🐛 [Report bugs](https://github.com/toolprint/vibe-workspace/issues)
- 💬 [Discussions](https://github.com/toolprint/vibe-workspace/discussions)
- 📖 [Documentation](https://github.com/toolprint/vibe-workspace/blob/main/README.md)

---
*Built with Rust 🦀 • Cross-compiled with Zig • Distributed via GitHub Releases*