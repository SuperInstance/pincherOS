# Contributing to PincherOS

Thank you for your interest in PincherOS! We welcome contributions from everyone.

## Code of Conduct

Be respectful. Be constructive. Be patient.

## Development Setup

```bash
# 1. Clone the repo
git clone https://github.com/SuperInstance/pincherOS.git
cd pincherOS

# 2. Build
cargo build

# 3. Run tests
cargo test

# 4. Run the example
cargo run --example teach_and_do

# 5. Try the CLI
./target/debug/pincher status
./target/debug/pincher seed
./target/debug/pincher teach "list files" "ls {path}"
./target/debug/pincher do "list files in /tmp"
```

## Good First Issues

Look for issues labeled `good first issue` on GitHub. These are small,
well-defined tasks that are great for getting familiar with the codebase.

## Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Make your changes
4. Ensure `cargo test` passes
5. Ensure `cargo clippy -- -D warnings` passes
6. Ensure `cargo fmt -- --check` passes
7. Submit a pull request

## Coding Standards

- **Rust**: Use `anyhow` for application errors, `thiserror` for library errors. No `unwrap()` in production code.
- **Python**: Follow PEP 8. Use type hints. Keep the sidecar stateless.
- **Documentation**: All public items must have rustdoc comments.
- **Tests**: Every new feature should include at least one test.

## Architecture

See [README.md](README.md) for the architecture overview and [docs/adr/](docs/adr/) for architecture decision records.
