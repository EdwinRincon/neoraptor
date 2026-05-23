# Default: format check + clippy + deny check
default:
    @just fmt-check
    @just clippy
    @just deny-check

# Lint: run all static checks (fmt + clippy + deny)
lint: fmt-check clippy deny-check

# Run cargo fmt --check
fmt-check:
    cargo fmt --all -- --check

# Run cargo clippy with workspace lints
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# Run cargo deny check (skip licenses for now - v0.1 foundation)
deny-check:
    cargo deny check advisories bans sources

# Auto-format all code
fmt:
    cargo fmt --all

# Run all tests
test:
    cargo test --workspace

# Check that everything compiles
check:
    cargo check --workspace

# Run dev server
dev:
    cargo run

# Prepare sqlx offline mode
prepare-sqlx:
    cargo sqlx prepare --workspace

# CI target: runs all checks
ci: fmt-check clippy deny-check check test