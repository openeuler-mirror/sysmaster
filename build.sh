#!/usr/bin/env bash
# Main build script for sysMaster
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FLAG_FILE="$SCRIPT_DIR/.git/hooks/commit-msg"

# Source cargo environment (required for both first and subsequent builds)
# shellcheck source=/dev/null
source "$HOME/.cargo/env"

# Check if this is the first build
if [[ -e "$FLAG_FILE" ]]; then
    echo "==> Not first build, skipping preinstall."
else
    echo "==> First build detected, running preinstall..."

    # Run all pre-installation scripts
    for script in ci/00-*.sh; do
        [[ -f "$script" ]] || continue
        echo "==> Running: $script"
        bash "$script"
    done

    # Re-source cargo environment after preinstall
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"

    touch "$FLAG_FILE"
    echo "==> Preinstall completed."
fi

# Execute test scripts (excluding 00-pre.sh)
echo "==> Running test scripts..."
for script in ci/*.sh; do
    [[ -f "$script" ]] || continue
    [[ "$(basename "$script")" == "00-pre.sh" ]] && continue

    echo "==> Running: $script"
    date
    bash "$script"
done

echo "==> All scripts completed successfully."
