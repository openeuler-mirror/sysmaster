#!/usr/bin/env bash
# Optional script: Build sysMaster with musl target
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=ci/common_function
source "$SCRIPT_DIR/common_function"

echo "==> (Optional) Building sysMaster with musl..."

# Install musl-gcc if not already installed
install_packages "musl-gcc"

# Get system architecture
arch=$(uname -m)

# Add musl target to Rust
echo "==> Adding musl target for $arch..."
rustup target add "$arch-unknown-linux-musl"

# Build with musl target
echo "==> Building with musl target..."
cargo build --all --no-default-features --features "default" --target="$arch-unknown-linux-musl"

echo "==> musl build completed successfully."
