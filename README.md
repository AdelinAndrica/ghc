# ghc — Interactive GitHub Repo Cloner

`ghc` is a fast, terminal-based tool that lets you **interactively browse and clone your GitHub repositories**.

No URLs.
No copy–paste.
Just search, select, and clone.

---

## Features

- Interactive TUI (arrow keys + Enter)
- Live search across repository names and descriptions
- Lists all repositories you have access to (personal + organizations)
- HTTPS or SSH cloning
- No GitHub CLI (`gh`) required
- Single native binary
- Cross-platform: Windows, macOS, Linux

---

## Installation

### Windows (Scoop)

```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
Invoke-RestMethod -Uri https://get.scoop.sh | Invoke-Expression
```

```powershell
scoop bucket add ghc https://github.com/AdelinAndrica/ghc-scoop
scoop install ghc
```

Upgrade later with:

```powershell
scoop update ghc
```

---

### macOS / Linux (Homebrew)

```bash
brew tap AdelinAndrica/ghc
brew install ghc
```

Upgrade later with:

```bash
brew upgrade ghc
```

---

### Manual install (any platform)

Download the appropriate binary from
[https://github.com/AdelinAndrica/ghc/releases](https://github.com/AdelinAndrica/ghc/releases)

Place it somewhere on your `PATH`.

---

## Authentication

`ghc` uses **GitHub OAuth Device Flow**, recommended for CLI tools.

Run once:

```bash
ghc login
```

You will:

1. Open a GitHub URL in your browser
2. Enter a short verification code
3. Approve access

After that, `ghc` works without further setup.

To log out:

```bash
ghc logout
```

To check auth status:

```bash
ghc auth-status
```

---

## Usage

Run:

```bash
ghc
```

### Controls

- ↑ / ↓ — move selection
- Type — filter repositories
- Backspace — delete filter character
- Enter — clone selected repository
- Esc / Ctrl+C — quit

Repositories are cloned into the current directory.

---

### Options

Clone using SSH instead of HTTPS:

```bash
ghc --ssh
```

Show only repositories you own:

```bash
ghc --owned
```

---

## Requirements

- Git (must be installed and available on PATH)

---

## What `ghc` does (and does not)

**Does:**

- Lists your GitHub repositories
- Runs `git clone` on the selected repo

**Does not:**

- Modify repositories
- Track usage or analytics
- Run background services
- Replace Git

---

## Security & Privacy

- Uses GitHub’s official OAuth Device Flow
- OAuth Client ID is public (as designed)
- Access token is stored locally on your machine
- No telemetry or external servers involved

---

## License

MIT
