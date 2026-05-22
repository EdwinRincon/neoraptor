default:
    @just --list

dev:
    cargo run

lint:
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings

test:
    cargo test

prepare-sqlx:
    cargo sqlx prepare --workspace