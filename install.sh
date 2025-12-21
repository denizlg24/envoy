#!/bin/bash
set -e


REPO_OWNER="denizlg24"
REPO_NAME="envoy"
BINARY_NAME="envy"


RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'


detect_platform() {
    local os=$(uname -s | tr '[:upper:]' '[:lower:]')
    local arch=$(uname -m)
    
    case "$os" in
        linux*)
            OS="linux"
            ;;
        darwin*)
            OS="macos"
            ;;
        *)
            echo -e "${RED}Unsupported operating system: $os${NC}"
            exit 1
            ;;
    esac
    
    case "$arch" in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        aarch64|arm64)
            ARCH="aarch64"
            ;;
        *)
            echo -e "${RED}Unsupported architecture: $arch${NC}"
            exit 1
            ;;
    esac
    

    if [ "$OS" = "linux" ]; then
        if [ "$ARCH" = "x86_64" ]; then
            TARGET="x86_64-unknown-linux-gnu"
        else
            TARGET="aarch64-unknown-linux-gnu"
        fi
    elif [ "$OS" = "macos" ]; then
        if [ "$ARCH" = "x86_64" ]; then
            TARGET="x86_64-apple-darwin"
        else
            TARGET="aarch64-apple-darwin"
        fi
    fi
}


get_install_dir() {
    if [ "$OS" = "macos" ] || [ "$OS" = "linux" ]; then

        if [ -d "$HOME/.local/bin" ] || mkdir -p "$HOME/.local/bin" 2>/dev/null; then
            INSTALL_DIR="$HOME/.local/bin"
        else
            INSTALL_DIR="$HOME/bin"
            mkdir -p "$INSTALL_DIR"
        fi
    fi
    BINARY_PATH="$INSTALL_DIR/$BINARY_NAME"
}


download_binary() {
    echo -e "${YELLOW}Fetching latest release...${NC}"
    

    local release_url="https://api.github.com/repos/$REPO_OWNER/$REPO_NAME/releases/latest"
    local download_url=""
    
    if command -v curl >/dev/null 2>&1; then
        download_url=$(curl -s "$release_url" | grep "browser_download_url.*${TARGET}" | cut -d '"' -f 4 | head -n 1)
    elif command -v wget >/dev/null 2>&1; then
        download_url=$(wget -qO- "$release_url" | grep "browser_download_url.*${TARGET}" | cut -d '"' -f 4 | head -n 1)
    else
        echo -e "${RED}Error: Neither curl nor wget is available${NC}"
        exit 1
    fi
    
    if [ -z "$download_url" ]; then
        echo -e "${RED}No release found for $TARGET${NC}"
        echo ""
        echo -e "${YELLOW}Alternative: Build from source:${NC}"
        echo "  1. Install Rust from https://rustup.rs/"
        echo "  2. Clone repo: git clone https://github.com/$REPO_OWNER/$REPO_NAME"
        echo "  3. Build: cd $REPO_NAME && cargo build --release"
        exit 1
    fi
    
    echo -e "${YELLOW}Downloading from: $download_url${NC}"
    
    local tmp_dir=$(mktemp -d)
    local archive_path="$tmp_dir/envoy-archive"
    
    if command -v curl >/dev/null 2>&1; then
        curl -L "$download_url" -o "$archive_path"
    else
        wget -q "$download_url" -O "$archive_path"
    fi
    

    echo -e "${YELLOW}Extracting...${NC}"
    if [[ "$download_url" == *.tar.gz ]]; then
        tar -xzf "$archive_path" -C "$tmp_dir"
    elif [[ "$download_url" == *.zip ]]; then
        unzip -q "$archive_path" -d "$tmp_dir"
    fi
    

    if [ -f "$tmp_dir/$BINARY_NAME" ]; then
        mv "$tmp_dir/$BINARY_NAME" "$BINARY_PATH"
        chmod +x "$BINARY_PATH"
        echo -e "${GREEN}Installed to: $BINARY_PATH${NC}"
    else
        echo -e "${RED}Error: Binary not found in archive${NC}"
        rm -rf "$tmp_dir"
        exit 1
    fi
    
    rm -rf "$tmp_dir"
}


update_path() {
    local shell_rc=""
    

    if [ -n "$BASH_VERSION" ]; then
        if [ -f "$HOME/.bashrc" ]; then
            shell_rc="$HOME/.bashrc"
        elif [ -f "$HOME/.bash_profile" ]; then
            shell_rc="$HOME/.bash_profile"
        fi
    elif [ -n "$ZSH_VERSION" ]; then
        shell_rc="$HOME/.zshrc"
    elif [ -f "$HOME/.profile" ]; then
        shell_rc="$HOME/.profile"
    fi
    

    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        echo -e "${YELLOW}Adding $INSTALL_DIR to PATH...${NC}"
        
        if [ -n "$shell_rc" ]; then
            echo "" >> "$shell_rc"
            echo "# Added by Envoy installer" >> "$shell_rc"
            echo "export PATH=\"\$PATH:$INSTALL_DIR\"" >> "$shell_rc"
            echo -e "${GREEN}Added to $shell_rc${NC}"
            echo -e "${YELLOW}Note: Restart your terminal or run: source $shell_rc${NC}"
        else
            echo -e "${YELLOW}Please manually add $INSTALL_DIR to your PATH${NC}"
        fi
    else
        echo -e "${GREEN}$INSTALL_DIR is already in PATH${NC}"
    fi
}


main() {
    echo -e "${CYAN}Installing Envoy CLI...${NC}"
    echo ""
    
    detect_platform
    echo -e "${CYAN}Detected: $OS ($ARCH) -> $TARGET${NC}"
    echo ""
    
    get_install_dir
    download_binary
    update_path
    
    echo ""
    echo -e "${GREEN}Installation complete!${NC}"
    echo ""
    echo -e "${CYAN}To get started:${NC}"
    echo "  $BINARY_NAME --help"
    echo ""
    
    if command -v "$BINARY_NAME" >/dev/null 2>&1; then
        echo -e "${GREEN}✓ $BINARY_NAME is ready to use${NC}"
    elif [ -x "$BINARY_PATH" ]; then
        echo -e "${YELLOW}Run the following to use envy in this session:${NC}"
        echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
    fi
}

main "$@"
