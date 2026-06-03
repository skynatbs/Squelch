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

# Push current branch (reads GITHUB_TOKEN from .env)
push:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -f .env ]; then source .env; fi
    TOKEN="${GITHUB_TOKEN:-}"
    if [ -n "$TOKEN" ]; then
        git push https://$TOKEN@github.com/skynatbs/Squelch.git HEAD
    else
        git push
    fi

# Stage, commit, and push in one step
ship message:
    #!/usr/bin/env bash
    set -euo pipefail
    git add -A
    git commit -m "{{message}}"
    if [ -f .env ]; then source .env; fi
    TOKEN="${GITHUB_TOKEN:-}"
    if [ -n "$TOKEN" ]; then
        git push https://$TOKEN@github.com/skynatbs/Squelch.git HEAD
    else
        git push
    fi

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

# ── Release ────────────────────────────────────────────────────────────────

# Tag setzen und pushen → löst den Windows-Release-Workflow aus
# Verwendung: just tag v0.1.0-alpha
tag version:
    #!/usr/bin/env bash
    set -euo pipefail
    git tag -a "{{version}}" -m "Release {{version}}"
    if [ -f .env ]; then source .env; fi
    TOKEN="${GITHUB_TOKEN:-}"
    if [ -n "$TOKEN" ]; then
        git push https://$TOKEN@github.com/skynatbs/Squelch.git "{{version}}"
    else
        git push origin "{{version}}"
    fi

# ── Misc ───────────────────────────────────────────────────────────────────

# Show dependency tree
tree:
    cargo tree

# Clean build artifacts
clean:
    cargo clean
