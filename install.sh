#!/bin/bash

# MediathekViewWeb CLI Installation Script
# Automatically detects OS and architecture and installs the appropriate binary

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
REPO="panjamo/mwb"
BINARY_NAME="mwb"
INSTALL_DIR="/usr/local/bin"
TEMP_DIR="/tmp/mwb-install"

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to detect OS and architecture
detect_platform() {
    local os=""
    local arch=""
    
    # Detect OS
    case "$(uname -s)" in
        Linux*)
            os="linux"
            # Check if we should use musl (Alpine Linux)
            if [ -f /etc/alpine-release ]; then
                os="linux-musl"
            fi
            ;;
        Darwin*)
            os="macos"
            ;;
        *)
            print_error "Unsupported operating system: $(uname -s)"
            exit 1
            ;;
    esac
    
    # Detect architecture
    case "$(uname -m)" in
        x86_64|amd64)
            arch="x64"
            ;;
        aarch64|arm64)
            arch="arm64"
            ;;
        *)
            print_error "Unsupported architecture: $(uname -m)"
            exit 1
            ;;
    esac
    
    echo "${os}-${arch}"
}

# Function to get latest release version
get_latest_version() {
    local latest_url="https://api.github.com/repos/${REPO}/releases/latest"
    
    if command -v curl >/dev/null 2>&1; then
        curl -s "${latest_url}" | grep '"tag_name":' | sed -E 's/.*"tag_name": "([^"]+)".*/\1/'
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- "${latest_url}" | grep '"tag_name":' | sed -E 's/.*"tag_name": "([^"]+)".*/\1/'
    else
        print_error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi
}

# Function to download file
download_file() {
    local url="$1"
    local output="$2"
    
    if command -v curl >/dev/null 2>&1; then
        curl -L -o "${output}" "${url}"
    elif command -v wget >/dev/null 2>&1; then
        wget -O "${output}" "${url}"
    else
        print_error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi
}

# Function to verify checksum
verify_checksum() {
    local file="$1"
    local checksum_file="$2"
    
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum -c "${checksum_file}"
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 -c "${checksum_file}"
    else
        print_warning "No checksum tool found. Skipping verification."
        return 0
    fi
}

# Function to check if running as root
check_permissions() {
    if [ -w "${INSTALL_DIR}" ]; then
        return 0
    elif [ "$(id -u)" -eq 0 ]; then
        return 0
    else
        print_warning "No write permission to ${INSTALL_DIR}"
        print_status "You may need to run this script with sudo or choose a different installation directory"
        
        # Offer alternative installation directory
        local user_bin="$HOME/.local/bin"
        echo -n "Install to ${user_bin} instead? (y/N): "
        read -r response
        if [ "${response}" = "y" ] || [ "${response}" = "Y" ]; then
            INSTALL_DIR="${user_bin}"
            mkdir -p "${INSTALL_DIR}"
            
            # Add to PATH if not already there
            if ! echo ":$PATH:" | grep -q ":$INSTALL_DIR:"; then
                print_status "Adding ${INSTALL_DIR} to PATH in your shell profile"
                
                # Detect shell and add to appropriate profile
                case "$SHELL" in
                    */bash)
                        echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
                        ;;
                    */zsh)
                        echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
                        ;;
                    */fish)
                        echo 'set -gx PATH $HOME/.local/bin $PATH' >> ~/.config/fish/config.fish
                        ;;
                    *)
                        print_warning "Unknown shell. Please add ${INSTALL_DIR} to your PATH manually."
                        ;;
                esac
                
                print_warning "Please restart your shell or run 'source ~/.bashrc' (or equivalent) to update PATH"
            fi
            return 0
        else
            print_error "Installation cancelled"
            exit 1
        fi
    fi
}

# Function to install binary
install_binary() {
    local platform="$1"
    local version="$2"
    local binary_name="mwb-${platform}"
    local download_url="https://github.com/${REPO}/releases/download/${version}/${binary_name}"
    local checksum_url="https://github.com/${REPO}/releases/download/${version}/${binary_name}.sha256"
    
    print_status "Installing MediathekViewWeb CLI ${version} for ${platform}..."
    
    # Create temporary directory
    mkdir -p "${TEMP_DIR}"
    cd "${TEMP_DIR}"
    
    # Download binary
    print_status "Downloading binary..."
    download_file "${download_url}" "${binary_name}"
    
    # Download and verify checksum
    print_status "Downloading checksum..."
    download_file "${checksum_url}" "${binary_name}.sha256"
    
    print_status "Verifying checksum..."
    if verify_checksum "${binary_name}" "${binary_name}.sha256"; then
        print_success "Checksum verification passed"
    else
        print_error "Checksum verification failed"
        exit 1
    fi
    
    # Make binary executable
    chmod +x "${binary_name}"
    
    # Install binary
    print_status "Installing to ${INSTALL_DIR}..."
    if command -v install >/dev/null 2>&1; then
        install -m 755 "${binary_name}" "${INSTALL_DIR}/${BINARY_NAME}"
    else
        cp "${binary_name}" "${INSTALL_DIR}/${BINARY_NAME}"
        chmod 755 "${INSTALL_DIR}/${BINARY_NAME}"
    fi
    
    # Clean up
    cd /
    rm -rf "${TEMP_DIR}"
    
    print_success "Installation completed successfully!"
}

# Function to test installation
test_installation() {
    if command -v "${BINARY_NAME}" >/dev/null 2>&1; then
        local installed_version
        installed_version=$("${BINARY_NAME}" --version 2>/dev/null | head -n1 || echo "unknown")
        print_success "MediathekViewWeb CLI is installed and accessible"
        print_status "Version: ${installed_version}"
        print_status "Location: $(command -v ${BINARY_NAME})"
        return 0
    else
        print_warning "Binary installed but not found in PATH"
        print_status "Try running: export PATH=\"${INSTALL_DIR}:\$PATH\""
        return 1
    fi
}

# Function to show usage examples
show_usage() {
    cat << 'EOF'

🎬 MediathekViewWeb CLI - Usage Examples:

Basic search:
  mwb search "documentary"
  mwb search "tatort" --size 10

VLC integration with quality selection:
  mwb search "news" --vlc          # Medium quality (default)
  mwb search "arte" --vlc=h        # HD quality
  mwb search "zdf" -v=l            # Low quality

Export formats:
  mwb search "science" --format json
  mwb search "culture" --format csv
  mwb search "sports" --format xspf

Advanced search:
  mwb search "documentary >60"     # Longer than 60 minutes
  mwb search "!ARD science"        # ARD channel, science topic
  mwb search "climate" --include "documentary|report"

For more options:
  mwb search --help
  mwb --help

EOF
}

# Main installation function
main() {
    echo "🎬 MediathekViewWeb CLI Installation Script"
    echo "==========================================="
    echo
    
    # Check prerequisites
    if ! command -v uname >/dev/null 2>&1; then
        print_error "uname command not found"
        exit 1
    fi
    
    # Detect platform
    local platform
    platform=$(detect_platform)
    print_status "Detected platform: ${platform}"
    
    # Check permissions
    check_permissions
    
    # Get latest version
    print_status "Fetching latest release information..."
    local version
    version=$(get_latest_version)
    if [ -z "${version}" ]; then
        print_error "Failed to get latest version"
        exit 1
    fi
    print_status "Latest version: ${version}"
    
    # Check if already installed
    if command -v "${BINARY_NAME}" >/dev/null 2>&1; then
        local current_version
        current_version=$("${BINARY_NAME}" --version 2>/dev/null | head -n1 | grep -o 'v[0-9]\+\.[0-9]\+\.[0-9]\+' || echo "unknown")
        if [ "${current_version}" = "${version}" ]; then
            print_success "MediathekViewWeb CLI ${version} is already installed"
            test_installation
            show_usage
            exit 0
        else
            print_status "Updating from ${current_version} to ${version}"
        fi
    fi
    
    # Install
    install_binary "${platform}" "${version}"
    
    # Test installation
    if test_installation; then
        show_usage
    else
        print_error "Installation verification failed"
        exit 1
    fi
}

# Handle command line arguments
case "${1:-}" in
    --help|-h)
        echo "MediathekViewWeb CLI Installation Script"
        echo
        echo "Usage: $0 [options]"
        echo
        echo "Options:"
        echo "  --help, -h     Show this help message"
        echo "  --version, -v  Show version information"
        echo "  --uninstall    Uninstall MediathekViewWeb CLI"
        echo
        echo "Environment variables:"
        echo "  INSTALL_DIR    Installation directory (default: /usr/local/bin)"
        echo "  REPO           GitHub repository (default: panjamo/mwb)"
        echo
        exit 0
        ;;
    --version|-v)
        local version
        version=$(get_latest_version)
        echo "Latest available version: ${version}"
        if command -v "${BINARY_NAME}" >/dev/null 2>&1; then
            local current_version
            current_version=$("${BINARY_NAME}" --version 2>/dev/null | head -n1 || echo "unknown")
            echo "Currently installed: ${current_version}"
        else
            echo "MediathekViewWeb CLI is not installed"
        fi
        exit 0
        ;;
    --uninstall)
        if [ -f "${INSTALL_DIR}/${BINARY_NAME}" ]; then
            print_status "Removing ${INSTALL_DIR}/${BINARY_NAME}..."
            rm -f "${INSTALL_DIR}/${BINARY_NAME}"
            print_success "MediathekViewWeb CLI uninstalled successfully"
        elif [ -f "$HOME/.local/bin/${BINARY_NAME}" ]; then
            print_status "Removing $HOME/.local/bin/${BINARY_NAME}..."
            rm -f "$HOME/.local/bin/${BINARY_NAME}"
            print_success "MediathekViewWeb CLI uninstalled successfully"
        else
            print_error "MediathekViewWeb CLI not found"
            exit 1
        fi
        exit 0
        ;;
    "")
        main
        ;;
    *)
        print_error "Unknown option: $1"
        echo "Use --help for usage information"
        exit 1
        ;;
esac