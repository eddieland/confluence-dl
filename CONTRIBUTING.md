# Contributing

So you want to contribute? Excellent. Here's what you need to know.

## Prerequisites

You'll need Rust. If you don't have it yet, install it from [rustup.rs](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

This project uses the nightly toolchain. Once you have `rustup` installed, the [`rust-toolchain.toml`](rust-toolchain.toml) file will automatically install and use the correct nightly version when you run cargo commands in this directory. You don't need to do anything special.

You'll also need `cargo-nextest` for running tests, because we don't use `cargo test` here. Install it:

```bash
cargo install cargo-nextest --locked
```

If you plan to check code coverage (you probably should), grab `cargo-llvm-cov`:

```bash
cargo install cargo-llvm-cov
```

## Development Workflow

We use a Makefile because typing the same commands repeatedly is tedious.

Start here:

```bash
make all
```

This runs formatting, linting, and tests. If it passes, you're probably good.

### Individual Commands

```bash
make build              # Build the debug version
make test               # Run tests (uses nextest, not cargo test)
make fmt                # Format code and auto-fix clippy issues
make lint               # Run clippy with strict settings
make release            # Build optimized binary
```

### About Tests

We use `cargo nextest` exclusively.

We also use snapshot testing via `insta`. If you change something that affects test snapshots:

```bash
make update-snapshots   # Update snapshots after changes
make insta-review       # Review snapshot changes interactively
```

## Code Style

We enforce formatting and linting automatically:

- **Line length**: 120 characters
- **Indentation**: 2 spaces (not 4)
- **MSRV**: 1.90.0
- **Cognitive complexity**: Keep functions under 25 complexity points
- **Function length**: Max 100 lines per function

Run `make fmt` before committing. Run `make lint` to catch issues. Or just run `make all` and save yourself the trouble.

## Submitting Changes

1. Fork the repository
2. Create a feature branch (not `main`)
3. Make your changes
4. Run `make all` and ensure it passes
5. Commit with a descriptive message
6. Push to your fork
7. Open a pull request

We're not picky about commit message formats, but "fix stuff" isn't helpful. Be specific.

## What to Contribute

Check the GitHub issues for things that need doing. If you want to add a feature that doesn't have an issue, open one first. Let's discuss it before you invest time.

Good first issues are typically labeled as such. Start there if you're new.

## Questions?

Open an issue. We're friendly, mostly.
