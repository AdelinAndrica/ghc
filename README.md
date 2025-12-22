# ghc — Interactive GitHub Repo Cloner

`ghc` is a fast, minimal terminal UI (TUI) that lets you interactively browse and clone your GitHub repositories using arrow keys, search, and Enter.

No URLs. No copy–paste. Just select and clone.

---

## Features

- Interactive terminal UI (arrow keys + Enter)
- Live search by name and description
- Shows all repositories you have access to (personal + organizations)
- HTTPS or SSH cloning
- Single native binary
- No runtime dependencies
- Cross-platform (Windows, macOS, Linux)

---

## Requirements

- Git
- GitHub CLI (`gh`)

---

## Authentication (one-time)

`ghc` reuses your GitHub CLI authentication.

```bash
gh auth login
```

Alternatively, set a token manually:

```bash
setx GITHUB_TOKEN ghp_xxxxxxxxxxxxxxxxxxxx
```

---

## Installation

### Prebuilt Binary (Recommended)

1. Download the binary for your OS from Releases
2. Place it in a directory on your PATH

Example (Windows):

```text
C:\Users\<you>\bin\ghc.exe
```

---

### Build from Source (Rust)

```bash
git clone https://github.com/<your-username>/ghc
cd ghc
cargo build --release
```

Copy the binary to your PATH:

```bash
copy target\release\ghc.exe %USERPROFILE%\bin\
```

---

## Usage

```bash
ghc
```

### Controls

- ↑ / ↓ — Navigate
- Type — Filter repositories
- Backspace — Delete filter character
- Enter — Clone selected repository
- Esc — Exit
- Ctrl+C — Exit immediately

The selected repository is cloned into the current directory.

---

### Clone using SSH

```bash
ghc --ssh
```

---

### Show only owned repositories

```bash
ghc --owned
```

---

## Troubleshooting

- Verify GitHub authentication: `gh auth status`
- Verify Git installation: `git --version`
- Verify binary location: `where ghc` (Windows) / `which ghc` (Unix)

---

## License

MIT
