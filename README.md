# NEORAPTOR: The Autonomous Offensive-Security OS

**NEO** references *The Matrix* — "the One" who sees the truth behind the illusion, like a pentester revealing the real state behind dashboards and firewalls.  
**RAPTOR** evokes a fast, precise predator, matching Rust's performance profile and the goal: hunt vulnerabilities quickly, safely, and intelligently.

NEORAPTOR is a foundation for continuous, AI-driven adversarial emulation. It orchestrates typed tool families over a replayable evidence graph, designed to behave like a senior, governed offensive operator with full operator-visible reasoning.

Unlike opaque AI agents or static scanners, NEORAPTOR is engineered in Rust with deterministic event-sourced runs and cryptographically verifiable scope contracts for forensic auditability and execution safety.

---

## Quick Start

### Prerequisites

- **Rust:** `1.82+` (see `rust-toolchain.toml`)
- **Docker:** For sandboxed tool execution
- **PostgreSQL:** Event log and artifact storage
- **Environment:** Linux or macOS (WSL2 supported)

### Build & Run

```bash
# Install dependencies
just install-deps

# Build the workspace
cargo build --workspace

# Run tests
cargo test --workspace

# Start the control plane (development)
cargo run --bin neoraptor-api

# Start a sandboxed worker
cargo run --bin neoraptor-worker
```

See `justfile` for additional commands.

---

## Architecture

For the full architectural design, including:

- System philosophy (Find/Fix/Verify/Visibility/Prioritization)
- Control Plane vs Execution Plane separation
- Core domain model (RunEvent, ScopeContract, ProbeSpec, EvidenceArtifact, CoverageMap)
- Typed autonomy loop and actor roles
- Tools, sandbox runtime, and governance

**→ Read [docs/architecture.md](docs/architecture.md)**

---

## Project Structure

```text
neoraptor-workspace/
├── crates/
│   ├── agents-core/          # Core primitives: RunEvent, ScopeContract, Evidence
│   ├── agents-impl/          # Typed Autonomy Loop actors
│   ├── neoraptor-api/        # HTTP API and composition root
│   ├── neoraptor-tools/      # Typed tool wrappers
│   ├── neoraptor-sandbox/    # Docker/gVisor isolation runtime
│   ├── config/               # Configuration types
│   ├── ports/                # Abstract capability traits
│   ├── providers/            # Concrete implementations
│   └── memory/               # Event log and evidence graph
├── docs/                     # Architecture and design docs
└── justfile                  # Build automation
```

---

## Development Workflow

1. **Create a branch** for your feature or fix
2. **Run tests** with `cargo test --workspace`
3. **Check lints** with `cargo clippy --workspace -- -D warnings`
4. **Format code** with `cargo fmt --all`
5. **Update `.sqlx/`** if adding DB queries: `just prepare-sqlx`
6. **Submit a PR** targeting `main`

---

## Documentation

- **[Architecture](docs/architecture.md)** — Full system design and implementation details
- **[Security](SECURITY.md)** — Security model, threat boundaries, and disclosure policy
- **[Work Plan](/docs/plan/INDEX.md)** — Roadmap and milestones for development
---

## Contributing

Contributions are welcome! Please:

- Follow the Rust API guidelines
- Respect the architectural boundaries (see [docs/architecture.md](docs/architecture.md))
- Add tests for new functionality
- Update documentation as needed

For questions or discussions, open an issue.