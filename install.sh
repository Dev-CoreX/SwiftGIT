#!/usr/bin/env bash
set -e

echo ""
echo "  ███████╗██╗    ██╗██╗███████╗████████╗ ██████╗ ██╗████████╗"
echo "  ██╔════╝██║    ██║██║██╔════╝╚══██╔══╝██╔════╝ ██║╚══██╔══╝"
echo "  ███████╗██║ █╗ ██║██║█████╗     ██║   ██║  ███╗██║   ██║   "
echo "  ╚════██║██║███╗██║██║██╔══╝     ██║   ██║   ██║██║   ██║   "
echo "  ███████║╚███╔███╔╝██║██║        ██║   ╚██████╔╝██║   ██║   "
echo "  ╚══════╝ ╚══╝╚══╝ ╚═╝╚═╝        ╚═╝    ╚═════╝ ╚═╝   ╚═╝   "
echo ""
echo "  SwiftGit v1 Installer"
echo ""

# Check system dependencies (Linux)
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    # Debian/Ubuntu (apt)
    if command -v apt-get &>/dev/null; then
        DEPS=("build-essential" "pkg-config" "git")
        MISSING_DEPS=()
        for dep in "${DEPS[@]}"; do
            if ! dpkg -l | grep -q "^ii  $dep "; then MISSING_DEPS+=("$dep"); fi
        done
        if [ ${#MISSING_DEPS[@]} -gt 0 ]; then
            echo "🔍 Missing system dependencies: ${MISSING_DEPS[*]}"
            read -p "❓ Install with apt? (y/n): " confirm
            [[ $confirm == [yY] ]] && sudo apt-get update && sudo apt-get install -y "${MISSING_DEPS[@]}"
        fi
    # Arch/Garuda (pacman)
    elif command -v pacman &>/dev/null; then
        DEPS=("base-devel" "pkg-config" "git")
        MISSING_DEPS=()
        for dep in "${DEPS[@]}"; do
            if ! pacman -Qi "$dep" &>/dev/null && ! pacman -Qg "$dep" &>/dev/null; then MISSING_DEPS+=("$dep"); fi
        done
        if [ ${#MISSING_DEPS[@]} -gt 0 ]; then
            echo "🔍 Missing system dependencies: ${MISSING_DEPS[*]}"
            read -p "❓ Install with pacman? (y/n): " confirm
            [[ $confirm == [yY] ]] && sudo pacman -S --needed "${MISSING_DEPS[@]}"
        fi
    fi
fi

# Check Cargo
if ! command -v cargo &>/dev/null; then
    echo "❌ Rust/Cargo not found. Install from https://rustup.rs"
    exit 1
fi

echo "✅ Dependencies ready"
echo "🔨 Building SwiftGit (release)..."

cargo build --release

INSTALL_DIR="${1:-/usr/local/bin}"
BINARY="./target/release/swiftgit"

if [ ! -f "$BINARY" ]; then
    echo "❌ Build failed — binary not found"
    exit 1
fi

echo "📦 Installing to $INSTALL_DIR/swiftgit ..."

if [ -w "$INSTALL_DIR" ]; then
    cp "$BINARY" "$INSTALL_DIR/swiftgit"
else
    sudo cp "$BINARY" "$INSTALL_DIR/swiftgit"
fi

echo ""
echo "✅ SwiftGit installed! Run: swiftgit"
echo ""
