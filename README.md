# deadbranch

**Clean up stale git branches safely.**

`deadbranch` helps you identify and remove old, unused git branches that clutter your repository. It's designed to be **safe by default** — protecting important branches and requiring explicit confirmation before any deletion.

## Features

- **List stale branches** — Find branches older than N days (default: 30)
- **Safe deletion** — Only deletes merged branches by default
- **Protected branches** — Never touches `main`, `master`, `develop`, `staging`, or `production`
- **WIP detection** — Automatically excludes `wip/*` and `draft/*` branches
- **Backup creation** — Saves deleted branch SHAs for easy restoration
- **Dry-run mode** — Preview what would be deleted without making changes
- **Local & remote** — Works with both local and remote branches

## Installation

### Quick Install (Linux/macOS)

```bash
curl -sSf https://raw.githubusercontent.com/armgabrielyan/deadbranch/main/install.sh | sh
```

### Homebrew (macOS/Linux)

```bash
brew install armgabrielyan/deadbranch/deadbranch
```

### npm/npx

```bash
# Install globally
npm install -g deadbranch

# Or run directly
npx deadbranch list
```

### Cargo (from source)

```bash
cargo install deadbranch
```

### Manual Download

Download pre-built binaries from the [GitHub Releases](https://github.com/armgabrielyan/deadbranch/releases) page.

| Platform | Architecture | Download |
|----------|--------------|----------|
| Linux | x86_64 (glibc) | `deadbranch-VERSION-x86_64-unknown-linux-gnu.tar.gz` |
| Linux | x86_64 (musl/static) | `deadbranch-VERSION-x86_64-unknown-linux-musl.tar.gz` |
| Linux | ARM64 | `deadbranch-VERSION-aarch64-unknown-linux-gnu.tar.gz` |
| macOS | Intel | `deadbranch-VERSION-x86_64-apple-darwin.tar.gz` |
| macOS | Apple Silicon | `deadbranch-VERSION-aarch64-apple-darwin.tar.gz` |
| Windows | x86_64 | `deadbranch-VERSION-x86_64-pc-windows-msvc.zip` |

### Build from Source

```bash
git clone https://github.com/armgabrielyan/deadbranch
cd deadbranch
cargo build --release
# Binary will be at target/release/deadbranch
```

## Quick Start

```bash
# List all stale branches (older than 30 days)
deadbranch list

# List branches older than 60 days
deadbranch list --days 60

# Preview what would be deleted
deadbranch clean --dry-run

# Delete merged stale branches (with confirmation)
deadbranch clean

# Delete only local branches
deadbranch clean --local
```

## Usage

### List Stale Branches

```bash
deadbranch list [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-d, --days <N>` | Only show branches older than N days (default: 30) |
| `--local` | Only show local branches |
| `--remote` | Only show remote branches |
| `--merged` | Only show merged branches |

**Example output:**

```
ℹ Using 'main' as the default branch for merge detection

Local Branches:
┌──────────────────────┬─────────┬────────┬──────────────┐
│ Branch               │ Age     │ Status │ Last Commit  │
├──────────────────────┼─────────┼────────┼──────────────┤
│ feature/old-api      │ 154d    │ merged │ 2024-09-01   │
│ bugfix/header-issue  │ 89d     │ merged │ 2024-11-03   │
└──────────────────────┴─────────┴────────┴──────────────┘

Remote Branches:
┌─────────────────────────────────┬─────────┬────────┬──────────────┐
│ Branch                          │ Age     │ Status │ Last Commit  │
├─────────────────────────────────┼─────────┼────────┼──────────────┤
│ origin/feature/deprecated       │ 203d    │ merged │ 2024-07-15   │
└─────────────────────────────────┴─────────┴────────┴──────────────┘
```

### Delete Stale Branches

```bash
deadbranch clean [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-d, --days <N>` | Only delete branches older than N days (default: 30) |
| `--merged` | Only delete merged branches (this is the default) |
| `--force` | Force delete unmerged branches (dangerous!) |
| `--dry-run` | Show what would be deleted without doing it |
| `--local` | Only delete local branches |
| `--remote` | Only delete remote branches |

**Safety features:**
- Only deletes **merged** branches by default
- Requires `--force` to delete unmerged branches
- Shows confirmation prompt before deletion
- Extra confirmation for remote branches
- Creates backup file with branch SHAs

**Example:**

```bash
$ deadbranch clean

Local Branches to Delete:
┌──────────────────────┬─────────┬────────┬──────────────┐
│ Branch               │ Age     │ Status │ Last Commit  │
├──────────────────────┼─────────┼────────┼──────────────┤
│ feature/old-api      │ 154d    │ merged │ 2024-09-01   │
│ bugfix/header-issue  │ 89d     │ merged │ 2024-11-03   │
└──────────────────────┴─────────┴────────┴──────────────┘

Delete 2 local branches? [y/N] y

Deleting local branches...
  ✓ feature/old-api
  ✓ bugfix/header-issue

✓ Deleted 2 local branches
  ↪ Backup: ~/.deadbranch/backups/my-repo/backup-20250201-143022.txt
```

### Dry Run Mode

Preview deletions without making any changes:

```bash
$ deadbranch clean --dry-run

Local Branches to Delete:
┌──────────────────────┬─────────┬────────┬──────────────┐
│ Branch               │ Age     │ Status │ Last Commit  │
├──────────────────────┼─────────┼────────┼──────────────┤
│ feature/old-api      │ 154d    │ merged │ 2024-09-01   │
└──────────────────────┴─────────┴────────┴──────────────┘

[DRY RUN] Commands that would be executed:
  git branch -d feature/old-api

No branches were actually deleted.
```

### Configuration

`deadbranch` stores its configuration in `~/.deadbranch/config.toml`.

```bash
# Show current configuration
deadbranch config show

# Set default age threshold
deadbranch config set days 45

# Set protected branches
deadbranch config set protected-branches main master develop

# Set exclude patterns
deadbranch config set exclude-patterns "wip/*" "draft/*" "temp/*"

# Open config in your editor
deadbranch config edit

# Reset to defaults
deadbranch config reset
```

**Default configuration:**

```toml
[general]
default_days = 30

[branches]
protected = ["main", "master", "develop", "staging", "production"]
exclude_patterns = ["wip/*", "draft/*", "*/wip", "*/draft"]
```

## Safety Features

`deadbranch` is designed to prevent accidental data loss:

| Feature | Description |
|---------|-------------|
| **Merged-only default** | Only deletes branches already merged to main/master |
| **Protected branches** | Never deletes main, master, develop, staging, production |
| **Current branch** | Never deletes the branch you're currently on |
| **WIP detection** | Excludes branches matching `wip/*`, `draft/*`, etc. |
| **Confirmation prompts** | Always asks before deleting |
| **Remote warning** | Extra confirmation for remote deletions |
| **Backup files** | Saves SHA of every deleted branch for restoration |
| **Dry-run mode** | Preview changes without risk |

## Restoring Deleted Branches

Every deletion creates a backup file at `~/.deadbranch/backups/<repo>/backup-<timestamp>.txt`.

The backup contains git commands to restore each branch:

```bash
# From the backup file:
git branch feature/old-api abc1234def5678
git branch bugfix/header-issue 987654fedcba
```

Simply run the appropriate command to restore a branch.

## Pattern Matching

Exclude patterns support glob-style wildcards:

| Pattern | Matches |
|---------|---------|
| `wip/*` | `wip/experiment`, `wip/test` |
| `*/draft` | `feature/draft`, `bugfix/draft` |
| `feature/*/temp` | `feature/foo/temp`, `feature/bar/temp` |
| `*test*` | `test`, `testing`, `my-test-branch` |

## Requirements

- Git (installed and accessible in PATH)
- A git repository (run from within a repo)

## Development

```bash
# Run tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Build release binary
cargo build --release

# Run clippy
cargo clippy
```

## License

MIT License — see [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request
