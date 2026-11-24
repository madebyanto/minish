#!/usr/bin/env bash
set -e

# The target, `-Zjson-target-spec` and `-Zbuild-std=core` come from `.cargo/config.toml`
# and the toolchain from `rust-toolchain.toml`, so a plain `cargo build` works too.
cargo clean && cargo build --release && echo "[OK] Build completed successfully." || echo "[ERROR] Build failed."
