#!/usr/bin/env bash
# Check if new dependencies are allowed
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ALLOW_FILE="$SCRIPT_DIR/crates.allow"

echo "==> Checking if new dependencies are allowed..."

# Get list of allowed crates from crates.allow file
if [[ ! -f "$ALLOW_FILE" ]]; then
    echo "Error: crates.allow file not found at $ALLOW_FILE" >&2
    exit 1
fi

ALLOWED_CRATES=$(grep -e "^-" "$ALLOW_FILE" | cut -d ':' -f 1 | cut -d '-' -f 2)

# Get new dependencies from Cargo.lock changes
# Handle case where origin/master doesn't exist
if ! git rev-parse --verify origin/master &>/dev/null; then
    echo "Warning: origin/master not found, skipping dependency check" >&2
    exit 0
fi

LOCK_FILES=()
readarray -t LOCK_FILES < <(git diff origin/master --name-only | grep '\.lock$')

DEPS=()
for lock_file in "${LOCK_FILES[@]}"; do
    while IFS= read -r dep; do
        DEPS+=("$dep")
    done < <(git diff "$lock_file" | grep -e "^+name =" | cut -d '=' -f 2 | xargs)
done

# Check each dependency against allowed list
for dep in "${DEPS[@]}"; do
    if ! grep -q "^$dep$" <<< "$ALLOWED_CRATES"; then
        echo "Error: Disallowed crate found: $dep" >&2
        exit 1
    fi
done

echo "==> All dependencies are allowed."
