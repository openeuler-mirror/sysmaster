#!/usr/bin/env bash
# Build all packages individually
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=ci/common_function
source "$SCRIPT_DIR/common_function"

# Install jq if not already installed
install_packages "jq"

echo "==> Building all packages individually..."

# Get list of all packages in workspace
while IFS= read -r line; do
    echo "==> Building package: $line"
    if ! cargo build --package "$line"; then
        echo "Error: Failed to build $line" >&2
        exit 1
    fi
done < <(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | "\(.name):\(.version)"')

echo "==> All packages built successfully."
