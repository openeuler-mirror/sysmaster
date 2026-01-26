#!/usr/bin/env bash
# Pre-installation script: Set up build environment
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=ci/common_function
source "$SCRIPT_DIR/common_function"

contains_chinese

# Install required packages using common function
required_packages=("gcc" "openssl-libs" "python3-pip" "python3" "python3-devel" "clang" "libblkid-devel" "kmod-devel" "libselinux-devel")
install_packages "${required_packages[@]}"

# Check if cargo is installed
cargo -v &>/dev/null
if [[ $? -ne 0 ]]; then
    echo "==> Installing Rust toolchain..."
    export RUSTUP_DIST_SERVER=https://mirrors.ustc.edu.cn/rust-static
    export RUSTUP_UPDATE_ROOT=https://mirrors.ustc.edu.cn/rust-static/rustup
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o rustlang.sh
    sh rustlang.sh -y --default-toolchain none
    rm -rf rustlang.sh
fi

# Source cargo environment
# shellcheck source=/dev/null
source "$HOME/.cargo/env"

# Set default Rust toolchain
if ! rustup default "$rust_vendor" 2>/dev/null; then
    echo "==> Installing Rust $rust_vendor..."
    rustup install "$rust_vendor"
    rustup default "$rust_vendor"
fi

# Find fastest cargo registry mirror
crate_names=(
    "https://github.com/rust-lang/crates.io-index"
    "https://mirrors.ustc.edu.cn/crates.io-index"
    "https://mirrors.tuna.tsinghua.edu.cn/git/crates.io-index.git"
    "https://mirrors.sjtug.sjtu.edu.cn/git/crates.io-index"
)

fastest_source=$(test_fasturl "${crate_names[@]}")
echo "==> Fastest cargo source: $fastest_source"

# Configure cargo to use fastest mirror
mkdir -p ~/.cargo
cat << EOF > ~/.cargo/config
[source.crates-io]
registry = "https://github.com/rust-lang/crates.io-index"

# Use the fastest source
replace-with = 'replace'

[source.replace]
registry = "$fastest_source"

[net]
git-fetch-with-cli = true
EOF

# Find fastest GitHub mirror
sources=("https://521github.com/" "https://gitclone.com/github.com/" "https://gh.api.99988866.xyz/https://github.com/" "https://github.com/")
url=$(test_fasturl "${sources[@]}")
git config --global url."${url}".insteadOf "https://github.com/"
echo "==> Using GitHub mirror: $url"

# Find fastest PyPI mirror
pipurls=("https://pypi.tuna.tsinghua.edu.cn/simple" "http://mirrors.aliyun.com/pypi/simple/" "https://pypi.mirrors.ustc.edu.cn/simple/" "http://pypi.sdutlinux.org/" "http://pypi.douban.com/simple/")
url=$(test_fasturl "${pipurls[@]}")

if [[ $url =~ ^https?://([^/]+) ]]; then
    domain="${BASH_REMATCH[1]}"
    pip config set global.index-url "$url"
    pip config set global.trusted-host "$domain"
    echo "==> Using PyPI mirror: $url"
fi

# Install pre-commit for local development
echo "==> Installing pre-commit and codespell..."
pip3 install --user pre-commit codespell
export PATH="$HOME/.local/bin:$PATH"

# Initialize pre-commit
echo "==> Initializing pre-commit hooks..."
git config --global init.templateDir ~/.git-template
pre-commit init-templatedir ~/.git-template
pre-commit install

echo "==> Pre-installation completed successfully."
