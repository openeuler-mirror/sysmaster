#!/usr/bin/env bash
# Pre-commit check script: Run linting and add lint attributes
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=ci/common_function
source "$SCRIPT_DIR/common_function"

# Cleanup function to remove deny lints on exit
finish() {
    echo "==> Removing temporary deny attributes..."
    set +x
    local rust_files

    if ! git rev-parse --verify origin/master &>/dev/null; then
        echo "Warning: origin/master not found, skipping cleanup" >&2
        rustup override unset
        return 0
    fi

    readarray -t rust_files < <(git diff origin/master --name-only | grep '\.rs$')

    for file in "${rust_files[@]}"; do
        [[ -f "$file" ]] || continue
        sed -i '/#!\[deny(missing_docs)]/d' "$file" 2>/dev/null || true
        sed -i '/#!\[deny(clippy::all)]/d' "$file" 2>/dev/null || true
        sed -i '/#!\[deny(warnings)]/d' "$file" 2>/dev/null || true
    done
    rustup override unset
}

trap finish EXIT

contains_chinese

# Ensure PATH includes user local bin for pre-commit
export PATH="$HOME/.local/bin:$PATH"

# Run cargo check
echo "==> Running cargo check..."
cargo check

# Add deny lint attributes to changed Rust files
echo "==> Adding deny lint attributes to changed files..."
rust_files=()
if git rev-parse --verify origin/master &>/dev/null; then
    readarray -t rust_files < <(git diff origin/master --name-only | grep '\.rs$' | grep -v "/examples/")
fi

for file in "${rust_files[@]}"; do
    [[ -f "$file" ]] || continue

    # Skip auto-generated files
    if [[ "$file" =~ "libblkid/mod.rs" ]] || [[ "$file" =~ "input_event_codes_rs" ]] || [[ "$file" =~ "proto/abi.rs" ]]; then
        continue
    fi

    # Restore commented-out allow attributes
    sed -i 's/\/\/#!\[allow(non_snake_case)\]/#![allow(non_snake_case)]/g' "$file" 2>/dev/null || true
    sed -i 's/\/\/#!\[allow(clippy::module_inception)\]/#![allow(clippy::module_inception)]/g' "$file" 2>/dev/null || true

    # Add deny lint attributes if not present
    grep -E '#!\[deny\(missing_docs\)\]' "$file" || sed -i '1i\#![deny(missing_docs)]' "$file" 2>/dev/null || true
    grep -E '#!\[deny\(clippy::all\)\]' "$file" || sed -i '1i\#![deny(clippy::all)]' "$file" 2>/dev/null || true
    grep -E '#!\[deny\(warnings\)\]' "$file" || sed -i '1i\#![deny(warnings)]' "$file" 2>/dev/null || true
done

# Run pre-commit checks
echo "==> Running pre-commit checks..."
pre-commit run -vvv --all-files
