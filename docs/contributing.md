# Contributing to PincherOS

First off, thank you for considering contributing to PincherOS. We're building something new — an operating system where AI agents learn, migrate, and live on real hardware — and we need your help.

---

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Development Environment Setup](#development-environment-setup)
- [Building and Running](#building-and-running)
- [Running Tests](#running-tests)
- [PR Process](#pr-process)
- [Coding Standards](#coding-standards)
- [Reporting Issues](#reporting-issues)
- [Feature Requests](#feature-requests)

---

## Code of Conduct

### Our Pledge

We pledge to make participation in PincherOS a harassment-free experience for everyone, regardless of age, body size, disability, ethnicity, sex characteristics, gender identity and expression, level of experience, education, socio-economic status, nationality, personal appearance, race, religion, or sexual identity and orientation.

### Our Standards

**Positive behavior includes:**
- Using welcoming and inclusive language
- Being respectful of differing viewpoints and experiences
- Gracefully accepting constructive criticism
- Focusing on what is best for the community
- Showing empathy toward other community members

**Unacceptable behavior includes:**
- The use of sexualized language or imagery
- Trolling, insulting/derogatory comments, and personal or political attacks
- Public or private harassment
- Publishing others' private information without explicit permission
- Other conduct which could reasonably be considered inappropriate

### Enforcement

Instances of abusive, harassing, or otherwise unacceptable behavior may be reported by contacting the project team at conduct@superinstance.com. All complaints will be reviewed and investigated and will result in a response that is deemed necessary and appropriate. The project team is obligated to maintain confidentiality with regard to the reporter of an incident.

---

## Development Environment Setup

### Prerequisites

| Tool | Version | Why |
|---|---|---|
| **Rust** | 1.75+ | Core runtime, CLI, CRDT engine |
| **Python** | 3.11+ | Inference sidecar (embeddings, LLM) |
| **Clang/LLVM** | 14+ | Required by some Rust dependencies on ARM |
| **bubblewrap** | Latest | Sandbox execution |
| **SQLite3** | 3.35+ | Metadata storage |
| **pkg-config** | Latest | Build dependency |

### Clone and Build

```bash
# Clone the repository
git clone https://github.com/SuperInstance/pincherOS.git
cd pincherOS

# Build the Rust workspace
cargo build

# Build in release mode (for benchmarking)
cargo build --release

# Set up the Python sidecar
cd sidecars/pincher-infer
python -m venv .venv
source .venv/bin/activate
pip install -e ".[dev]"

# Download models (first time only — ~700MB)
pincher download-model tinyllama-1.1b-q4_km
pincher download-model minilm-l6
```

### Cross-Compilation for ARM

If you're developing on x86_64 and targeting Raspberry Pi:

```bash
# Add ARM target
rustup target add aarch64-unknown-linux-gnu

# Install cross-compilation toolchain
sudo apt install gcc-aarch64-linux-gnu

# Build for ARM
cargo build --target aarch64-unknown-linux-gnu
```

For Jetson Nano (aarch64 + CUDA):

```bash
# Install NVIDIA cross-compilation tools
# Follow NVIDIA's JetPack SDK instructions

# Build with CUDA feature
cargo build --target aarch64-unknown-linux-gnu --features cuda
```

---

## Building and Running

### Start the Agent

```bash
# Initialize (first time)
pincher init

# Start the agent
pincher start

# In another terminal, interact with it
pincher "list the largest files in my Downloads"
```

### Development Mode

```bash
# Run with debug logging
RUST_LOG=pincher_core=debug,pincher_infer=debug pincher start

# Run with tracing (for performance analysis)
RUST_LOG=trace pincher start 2> trace.log

# Run the Rust core only (no Python sidecar)
pincher start --no-infer

# Run the Python sidecar standalone (for debugging)
cd sidecars/pincher-infer
python -m pincher_infer.server --socket /tmp/test-infer.sock
```

---

## Running Tests

### Rust Tests

```bash
# Run all Rust tests
cargo test

# Run tests for a specific crate
cargo test -p pincher-core
cargo test -p pincher-crdt

# Run tests with CUDA features (requires GPU)
cargo test --features cuda

# Run a specific test
cargo test -p pincher-core test_reflex_short_circuit

# Run benchmarks
cargo bench -p pincher-crdt
```

### Python Tests

```bash
cd sidecars/pincher-infer

# Run all Python tests
pytest

# Run with coverage
pytest --cov=pincher_infer --cov-report=html

# Run a specific test file
pytest tests/test_embedding.py -v

# Run integration tests (requires models downloaded)
pytest tests/integration/ -v --run-integration
```

### Integration Tests

```bash
# Full end-to-end test (requires models + bwrap)
cargo test -p pincher-cli --test e2e

# Migration round-trip test
cargo test -p pincher-core test_migration_round_trip
```

---

## PR Process

### Before You Start

1. **Check existing issues and PRs** — someone may already be working on it
2. **Open an issue** for significant changes — get feedback before you write code
3. **Small PRs are better** — aim for < 400 lines of changes per PR

### PR Checklist

- [ ] Code compiles without warnings (`cargo build` and `cargo clippy`)
- [ ] All tests pass (`cargo test` and `pytest`)
- [ ] Code is formatted (`cargo fmt` and `ruff format`)
- [ ] Type checks pass (`mypy` for Python)
- [ ] New code has tests
- [ ] Documentation is updated (if changing public APIs)
- [ ] Commit messages are clear and descriptive

### PR Title Format

```
type(scope): description

Examples:
  feat(rigging): add reflex compaction on migration
  fix(sandbox): resolve bwrap bind mount ordering
  docs(architecture): add shell epigenetics section
  test(crdt): add hot/cold partition benchmarks
  refactor(core): simplify confidence update formula
```

### Types

| Type | Meaning |
|---|---|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `test` | Adding or updating tests |
| `refactor` | Code restructuring without behavior change |
| `perf` | Performance improvement |
| `chore` | Maintenance (CI, dependencies, etc.) |

### Review Process

1. Automated checks must pass (CI builds + tests + lint)
2. At least one maintainer review required
3. All conversations must be resolved before merge
4. Squash merge to `main` (clean history)

---

## Coding Standards

### Rust

**Formatting:**
```bash
# Format all Rust code
cargo fmt --all

# Check formatting without modifying files
cargo fmt --all -- --check
```

**Linting:**
```bash
# Run Clippy (Rust linter)
cargo clippy --all-targets --all-features -- -D warnings

# Run Clippy with stricter settings
cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic
```

**Key conventions:**
- Use `anyhow::Result` for application code, custom error types for library code
- Prefer `tokio` async runtime; never block the executor
- Use `tracing` for structured logging, not `println!`
- All public types must derive or implement `Debug`
- Document all public APIs with `///` doc comments
- Use `#[cfg(feature = "cuda")]` for GPU-specific code — never unconditionally require CUDA
- Prefer `DashMap` over `Mutex<HashMap>` for concurrent access patterns
- Use `Uuid::now_v7()` for time-ordered UUIDs (not v4)

**Error handling:**
```rust
// GOOD: Specific error type with context
#[derive(Debug, thiserror::Error)]
pub enum ReflexError {
    #[error("Reflex {id} not found")]
    NotFound { id: ReflexId },
    #[error("Confidence {confidence} below threshold {threshold}")]
    BelowThreshold { confidence: f64, threshold: f64 },
}

// BAD: Stringly-typed errors
fn get_reflex(id: &ReflexId) -> Result<Reflex, String> { ... }
```

**Ownership patterns:**
```rust
// Shell = borrowed (the agent doesn't own the hardware)
// Rigging = owned (the agent IS the rigging)
// Claws = borrowed (compute substrate is shared)
// Exoskeleton = projected (rendered from rigging state)

// This maps to Rust's ownership model naturally:
pub struct Agent {
    rigging: Rigging,                         // Owned
    shell: ShellProfile,                      // Borrowed snapshot
    claws: Box<dyn Claws>,                    // Borrowed via trait object
}
```

### Python

**Formatting:**
```bash
# Format all Python code
ruff format sidecars/

# Check formatting
ruff format --check sidecars/
```

**Linting:**
```bash
# Run Ruff (fast Python linter, replaces flake8 + isort)
ruff check sidecars/

# Auto-fix issues
ruff check --fix sidecars/
```

**Type checking:**
```bash
# Run MyPy
mypy sidecars/pincher_infer/
```

**Key conventions:**
- Use `pydantic` for all data models — no raw dicts for structured data
- Use `asyncio` for all I/O — never block the event loop
- Use `structlog` for structured logging
- All public functions must have type hints
- Keep the Python sidecar thin — it's an inference bridge, not a business logic layer
- The Rust core is the source of truth for all state — Python reads and writes through the UDS protocol

**Model loading:**
```python
# GOOD: Lazy-load, unload when idle
class InferenceServer:
    def __init__(self):
        self._llm = None  # Not loaded until first request
        self._last_inference = 0
        self._unload_after_s = 300

    @property
    def llm(self):
        if self._llm is None:
            self._llm = Llama(model_path=MODEL_PATH, ...)
        self._last_inference = time.time()
        return self._llm

# BAD: Eager-load on import
llm = Llama(model_path=MODEL_PATH, ...)  # 1.1GB of RAM gone at startup
```

---

## Reporting Issues

### Bug Reports

Please include:

1. **Hardware**: Device type (RPi 4, Jetson Nano, workstation)
2. **OS**: Linux distribution and kernel version
3. **PincherOS version**: Output of `pincher --version`
4. **Steps to reproduce**: What you did, what you expected, what happened
5. **Logs**: Relevant output from `pincher start` or the log file at `/opt/pincheros/run/pincher.log`

### Security Issues

**Do not file security issues as public GitHub issues.** Email security@superinstance.com instead. We will respond within 48 hours and work with you to resolve the issue before public disclosure.

---

## Feature Requests

We welcome feature requests, especially for:

- New shell types (ESP32, Mac M-series, cloud instances)
- New reflex sources (distilling from API docs, learning from shell history)
- New Claws implementations (Vulkan compute, Apple Metal, WebGPU)
- UI improvements (TUI, web dashboard, voice interface)
- New CRDT types and merge strategies

Please open an issue with the label `feature-request` and describe:
1. What you want to do
2. Why existing features don't support it
3. Any ideas you have for implementation

---

## License

By contributing to PincherOS, you agree that your contributions will be licensed under the [MIT License](../LICENSE).

---

Thank you for helping build the operating system where agents learn, migrate, and belong. Same crab, bigger shell. 🦀
