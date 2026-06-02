# Squelch Justfile
# https://just.systems

# Default: list available recipes
default:
    @just --list

# ── Build ──────────────────────────────────────────────────────────────────

# Build entire workspace (debug)
build:
    cargo build

# Build release
build-release:
    cargo build --release

# Build a single crate
build-crate crate:
    cargo build -p {{crate}}

# ── Test ───────────────────────────────────────────────────────────────────

# Run all tests
test:
    cargo test

# Run tests for a single crate
test-crate crate:
    cargo test -p {{crate}}

# Run a specific test by name
test-one crate name:
    cargo test -p {{crate}} {{name}}

# ── Code quality ───────────────────────────────────────────────────────────

# Run clippy on entire workspace
lint:
    cargo clippy --workspace -- -D warnings

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Apply formatting
fmt:
    cargo fmt --all

# Check licenses and security advisories (requires cargo-deny)
deny:
    cargo deny check

# All quality gates: lint + fmt-check + test
check: lint fmt-check test

# ── Git ────────────────────────────────────────────────────────────────────

# Stage all and commit (usage: just commit "message")
commit message:
    git add -A
    git commit -m "{{message}}"

# Push current branch
push:
    git push

# Stage, commit, and push in one step
ship message:
    git add -A
    git commit -m "{{message}}"
    git push

# Show compact log
log:
    git log --oneline -20

# ── Spikes ─────────────────────────────────────────────────────────────────

# Build a spike (usage: just spike 001-str0m-webrtc)
spike name:
    cargo build --manifest-path spikes/{{name}}/Cargo.toml

# Run a spike
run-spike name:
    cargo run --manifest-path spikes/{{name}}/Cargo.toml

# ── Misc ───────────────────────────────────────────────────────────────────

# Show dependency tree
tree:
    cargo tree

# Clean build artifacts
clean:
    cargo clean
