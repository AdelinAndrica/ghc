## v0.0.4 — Stability & UX Polish

- Fix double keypress handling across terminals
- Improve TUI redraw logic to eliminate flicker
- Graceful GitHub API/network error messages
- Automatic retry on transient network failures
- Validate stored OAuth token on startup
- Clear re-auth prompt when token is invalid or expired
- Hide archived repositories by default
- Show total repository count before filtering
- Add `--version` flag (version, commit, target)
- Add basic CI sanity check (`ghc --help`)
- Minor help text and usage cleanup

---

## v0.0.5 — Discoverability & Sorting

- Client-side sorting options:
  - last updated (default)
  - name
  - stars
- Add `--sort` CLI flag
- Optional display of archived repositories via `--archived`
- Improve repository metadata display (stars, forks count)
- Better empty-state messages when no repos match search

---

## v0.0.6 — Productivity & Flow

- Remember last search query between runs
- Faster initial render for large repo lists
- Keyboard shortcuts help overlay (`?`)
- Visual indicator for private, forked, and archived repos
- Consistent keybindings across platforms

---

## v0.0.7 — CLI & Scriptability

- Non-interactive clone mode:

```bash
  ghc clone owner/repo
```

- Support `--https` / `--ssh` explicitly
- Exit codes suitable for scripting
- Clear stdout/stderr separation
- Improved error messages for `git clone` failures

---

## v0.0.8 — Authentication & Config

- Clearer OAuth device-flow instructions
- Explicit scope documentation
- Better auth-status output
- Config file versioning and migration
- Safer handling of corrupted config/auth files

---

## v0.0.9 — Packaging & Reliability

- Harden Homebrew and Scoop install paths
- Improve release artifact verification
- Faster CI builds via caching improvements
- Cross-platform consistency checks
- Expanded smoke tests for release binaries

---

## v0.1.0 — Stable CLI Release

- Stable CLI interface (no breaking changes within 0.1.x)
- Fully documented installation paths (Brew, Scoop, Manual)
- Polished TUI with consistent UX
- Robust authentication and error handling
- Reliable, reproducible release pipeline
- Ready for submission to Homebrew Core and Scoop Main
