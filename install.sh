#!/usr/bin/env bash
set -e

# ── Colors & Styles ───────────────────────────────────────────────────────────
ORANGE='\033[38;5;208m'
NC='\033[0m' # No Color
BOLD='\033[1m'
DIM='\033[2m'
CYAN='\033[36m'
GREEN='\033[32m'
RED='\033[31m'

# ── Helper Functions ──────────────────────────────────────────────────────────

draw_box() {
    local title="$1"
    local width=60
    local padding=$(( (width - ${#title} - 2) / 2 ))
    
    echo -e "${ORANGE}┌$(printf '─%.0s' $(seq 1 $width))┐${NC}"
    printf "${ORANGE}│${NC}%*s${BOLD}${ORANGE}%s${NC}%*s${ORANGE}│${NC}\n" $padding "" "$title" $((width - padding - ${#title})) ""
    echo -e "${ORANGE}└$(printf '─%.0s' $(seq 1 $width))┘${NC}"
}

print_step() {
    echo -e "\n${BOLD}${ORANGE}➤ $1${NC}"
}

# ── Header ────────────────────────────────────────────────────────────────────

clear
echo -e "${ORANGE}"
echo "  ███████╗██╗    ██╗██╗███████╗████████╗ ██████╗ ██╗████████╗"
echo "  ██╔════╝██║    ██║██║██╔════╝╚══██╔══╝██╔════╝ ██║╚══██╔══╝"
echo "  ███████╗██║ █╗ ██║██║█████╗     ██║   ██║  ███╗██║   ██║   "
echo "  ╚════██║██║███╗██║██║██╔══╝     ██║   ██║   ██║██║   ██║   "
echo "  ███████║╚███╔███╔╝██║██║        ██║   ╚██████╔╝██║   ██║   "
echo "  ╚══════╝ ╚══╝╚══╝ ╚═╝╚═╝        ╚═╝    ╚═════╝ ╚═╝   ╚═╝   "
echo -e "${NC}"

draw_box "SwiftGit v1.4 Installer & Setup"

# ── Check Dependencies ────────────────────────────────────────────────────────

print_step "Checking system dependencies..."
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    if command -v pacman &>/dev/null; then
        sudo pacman -S --needed --noconfirm base-devel pkg-config git
    elif command -v apt-get &>/dev/null; then
        sudo apt-get update && sudo apt-get install -y build-essential pkg-config git
    fi
fi

if ! command -v cargo &>/dev/null; then
    echo -e "${RED}❌ Rust/Cargo not found. Install from https://rustup.rs${NC}"
    exit 1
fi
echo -e "${GREEN}✅ System ready!${NC}"

# ── Build & Install ───────────────────────────────────────────────────────────

print_step "Building SwiftGit (Release mode)..."
cargo build --release

INSTALL_DIR="/usr/local/bin"
BINARY="./target/release/swiftgit"

if [ ! -f "$BINARY" ]; then
    echo -e "${RED}❌ Build failed!${NC}"
    exit 1
fi

echo -e "${ORANGE}📦 Installing to $INSTALL_DIR/swiftgit...${NC}"
if [ -w "$INSTALL_DIR" ]; then
    cp "$BINARY" "$INSTALL_DIR/swiftgit"
else
    sudo cp "$BINARY" "$INSTALL_DIR/swiftgit"
fi
echo -e "${GREEN}✅ Binary installed!${NC}"

# ── Configuration Wizard ──────────────────────────────────────────────────────

echo ""
draw_box "Configuration Wizard"
echo -e "${DIM}This will set up your GitHub credentials and SSH keys.${NC}"

# 1. GitHub Username
print_step "1/4: GitHub Username"
read -p "   Enter your GitHub username: " GH_USER
while [ -z "$GH_USER" ]; do
    read -p "   ⚠️ Username cannot be empty: " GH_USER
done

# 2. Display Name
print_step "2/4: Display Name"
read -p "   Enter your display name (e.g. Real Name): " DISP_NAME
if [ -z "$DISP_NAME" ]; then DISP_NAME="$GH_USER"; fi

# 3. GitHub PAT
print_step "3/4: Personal Access Token (PAT)"
echo -e "   ${DIM}Generate at: https://github.com/settings/tokens (repo scope)${NC}"
read -p "   Paste your token: " GH_TOKEN
while [ -z "$GH_TOKEN" ]; do
    read -p "   ⚠️ Token is required for push/pull: " GH_TOKEN
done

# 4. SSH Setup
print_step "4/4: SSH Key Configuration"
SSH_DIR="$HOME/.ssh"
SSH_KEY_FOUND=false
SSH_PUB_PATH=""

# Check for existing keys
for key in id_ed25519 id_rsa; do
    if [ -f "$SSH_DIR/$key.pub" ]; then
        SSH_KEY_FOUND=true
        SSH_PUB_PATH="$SSH_DIR/$key.pub"
        break
    fi
done

if [ "$SSH_KEY_FOUND" = true ]; then
    echo -e "   ${GREEN}✅ Found existing SSH key: $SSH_PUB_PATH${NC}"
    read -p "   ❓ Use this key for SwiftGit? (y/n): " USE_EXISTING
else
    echo -e "   ${RED}❌ No SSH key found.${NC}"
    USE_EXISTING="n"
fi

SSH_ADDED=false
if [[ $USE_EXISTING == [yY] ]]; then
    SSH_ADDED=true
else
    read -p "   ❓ Generate a new ed25519 SSH key? (y/n): " GEN_NEW
    if [[ $GEN_NEW == [yY] ]]; then
        echo -e "   ${ORANGE}⚙️ Generating key...${NC}"
        ssh-keygen -t ed25519 -C "$GH_USER@swiftgit" -f "$SSH_DIR/id_ed25519" -N ""
        SSH_PUB_PATH="$SSH_DIR/id_ed25519.pub"
        echo -e "   ${GREEN}✅ Key generated!${NC}"
        echo -e "\n   ${BOLD}${CYAN}IMPORTANT:${NC} Copy this public key and add it to GitHub Settings > SSH keys:"
        echo -e "   ${ORANGE}$(cat "$SSH_PUB_PATH")${NC}\n"
        read -p "   Press Enter once you have added it to GitHub... "
        SSH_ADDED=true
    fi
fi

# ── Save Config ───────────────────────────────────────────────────────────────

print_step "Saving configuration..."
CONFIG_DIR="$HOME/.swiftgit"
mkdir -p "$CONFIG_DIR"

# Build JSON manually to avoid jq dependency
cat > "$CONFIG_DIR/config.json" <<EOF
{
  "github_token": "$GH_TOKEN",
  "username": "$GH_USER",
  "display_name": "$DISP_NAME",
  "ssh_key_added": $SSH_ADDED,
  "recent_projects": []
}
EOF

echo -e "${GREEN}✅ Configuration saved to $CONFIG_DIR/config.json${NC}"

# ── Final Finish ──────────────────────────────────────────────────────────────

echo ""
draw_box "Installation Complete!"
echo -e "\n  ${BOLD}${GREEN}SwiftGit v1.4 is ready to use!${NC}"
echo -e "  Type ${BOLD}${ORANGE}swiftgit${NC} to launch.\n"