#!/usr/bin/env bash
set -euo pipefail

echo "[pre-commit] cargo fmt --all --check"
cargo fmt --all --check

echo "[pre-commit] cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

echo "[pre-commit] cargo test --all-targets --locked"
cargo test --all-targets --locked

echo "[pre-commit] cargo test --locked --no-default-features --features lua54"
cargo test --locked --no-default-features --features lua54

echo "[pre-commit] build and run example"
pushd examples/simple
cargo run -- bundle lua/main.lua --lua "luajit"
command -v luajit >/dev/null && luajit dist/simple_bundle.lua || echo 'luajit not found, skipping' 
popd

echo "[pre-commit] OK"
