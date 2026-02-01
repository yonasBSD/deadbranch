# deadbranch npm package

This is the npm distribution package for [deadbranch](https://github.com/armgabrielyan/deadbranch), a CLI tool to clean up stale git branches safely.

## Installation

```bash
npm install -g deadbranch
```

Or run directly with npx:

```bash
npx deadbranch list
```

## What this package does

When you install this package, it automatically downloads the appropriate pre-built binary for your platform from GitHub Releases. Supported platforms:

- macOS (Intel & Apple Silicon)
- Linux (x64 & ARM64)
- Windows (x64)

## Alternative installation methods

If npm installation fails, you can install deadbranch using other methods:

### Shell installer (Linux/macOS)
```bash
curl -sSf https://raw.githubusercontent.com/armgabrielyan/deadbranch/main/install.sh | sh
```

### Cargo (from source)
```bash
cargo install deadbranch
```

### Homebrew (macOS/Linux)
```bash
brew install armgabrielyan/deadbranch/deadbranch
```

## Usage

```bash
deadbranch list              # List stale branches
deadbranch clean             # Delete merged stale branches
deadbranch clean --dry-run   # Preview what would be deleted
```

For more information, see the [full documentation](https://github.com/armgabrielyan/deadbranch).

## License

MIT
