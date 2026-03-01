# Contributing to deadbranch

Thank you for your interest in contributing! This guide covers everything you need to get started.

## Table of Contents

- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Commit Messages](#commit-messages)
- [Testing](#testing)
- [Submitting a Pull Request](#submitting-a-pull-request)
- [Code Style](#code-style)

## Development Setup

**Prerequisites:** Rust toolchain (stable) and Git installed.

```bash
git clone https://github.com/armgabrielyan/deadbranch
cd deadbranch
cargo build
```

Useful commands:

```bash
cargo build            # Debug build
cargo build --release  # Release build
cargo test             # Run all tests
cargo clippy           # Lint
cargo fmt              # Format code
```

## Making Changes

1. Fork the repository
2. Create a feature branch: `git checkout -b feat/my-feature`
3. Make your changes
4. Add tests covering your changes (see [Testing](#testing))
5. Ensure all tests pass: `cargo test`
6. Ensure there are no lint warnings: `cargo clippy`
7. Format your code: `cargo fmt`
8. Commit following the [commit message conventions](#commit-messages)
9. Open a pull request

## Commit Messages

This project follows the [Conventional Commits](https://www.conventionalcommits.org/) specification. The CHANGELOG and releases are generated automatically from commit messages, so following this format is required.

### Format

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

### Types

| Type | When to use |
|------|-------------|
| `feat` | A new feature |
| `fix` | A bug fix |
| `docs` | Documentation changes only |
| `test` | Adding or updating tests |
| `refactor` | Code change that is neither a fix nor a feature |
| `perf` | Performance improvements |
| `chore` | Build process, dependency updates, tooling |
| `ci` | CI/CD configuration changes |

### Examples

```
feat: add --only-mine flag to filter personal branches
fix: exclude current branch from remote deletion
docs: update shell completion instructions
test: add integration tests for backup restore command
refactor: extract branch age formatting into helper
```

### Breaking Changes

Append `!` after the type or add `BREAKING CHANGE:` in the footer:

```
feat!: rename --force flag to --unmerged

BREAKING CHANGE: --force has been renamed to --unmerged for clarity.
```

## Testing

All new functionality and bug fixes **must** include tests.

### Test structure

| Location | What to test |
|----------|--------------|
| `src/<module>.rs` (unit tests) | Pure logic: filtering, parsing, formatting, config operations |
| `tests/cli_tests.rs` | CLI commands end-to-end via a real temporary git repo |
| `tests/edge_case_tests.rs` | Edge cases: current branch exclusion, age display, multiple branches |

### Running tests

```bash
# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test cli_tests
cargo test --test edge_case_tests
cargo test --test backup_tests

# Run a specific test by name
cargo test test_branch_is_protected

# Show stdout from tests
cargo test -- --nocapture
```

### What to test

- **New flags or options**: add a CLI integration test that exercises the flag in a real git repo
- **Filtering/sorting logic**: add unit tests in the relevant module
- **Bug fixes**: add a test that would have caught the bug
- **Config changes**: add tests for the new config key and its validation

### What you don't need to test

- Interactive TTY prompts (confirmation dialogs)
- Remote push/pull operations (requires network)
- `config edit` (requires `$EDITOR`)

## Submitting a Pull Request

- Keep PRs focused — one feature or fix per PR
- Reference any related issues in the PR description
- Ensure CI passes before requesting review
- A maintainer will review and merge your PR

## Code Style

- Run `cargo fmt` before committing — the CI will reject unformatted code
- Address all `cargo clippy` warnings
- Public functions and types must have doc comments
- Only add inline comments where the logic is non-obvious
