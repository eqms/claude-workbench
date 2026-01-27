#!/bin/bash
#
# install.sh - Install claude-workbench on Linux/macOS
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/eqms/claude-workbench/main/scripts/install.sh | bash
#   bash scripts/install.sh [OPTIONS]
#
# Options:
#   --help, -h         Show this help message
#   --version          Show script version
#   --local            Build from source using cargo (requires Git repo)
#   --install-dir DIR  Installation directory (default: ~/.local/bin)
#   --check            Only check dependencies, don't install
#

set -euo pipefail

# Script version
SCRIPT_VERSION="1.0.0"

# Configuration
BINARY_NAME="claude-workbench"
REPO="eqms/claude-workbench"
GITHUB_URL="https://github.com/${REPO}"
INSTALL_DIR="${HOME}/.local/bin"
LOCAL_BUILD=false
CHECK_ONLY=false
TMPDIR_CLEANUP=""

# Colors (using printf-compatible format)
if [ -t 1 ]; then
    RED=$(printf '\033[0;31m')
    GREEN=$(printf '\033[0;32m')
    YELLOW=$(printf '\033[1;33m')
    BLUE=$(printf '\033[0;34m')
    CYAN=$(printf '\033[0;36m')
    BOLD=$(printf '\033[1m')
    DIM=$(printf '\033[2m')
    NC=$(printf '\033[0m')
else
    RED="" GREEN="" YELLOW="" BLUE="" CYAN="" BOLD="" DIM="" NC=""
fi

# Cleanup handler
cleanup() {
    if [[ -n "$TMPDIR_CLEANUP" && -d "$TMPDIR_CLEANUP" ]]; then
        rm -rf "$TMPDIR_CLEANUP"
    fi
}
trap cleanup EXIT

# --- Argument Parsing ---

parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --help|-h)
                print_help
                exit 0
                ;;
            --version)
                echo "install.sh version ${SCRIPT_VERSION}"
                exit 0
                ;;
            --local)
                LOCAL_BUILD=true
                shift
                ;;
            --install-dir)
                if [[ -z "${2:-}" ]]; then
                    printf "%sError: --install-dir requires a path argument%s\n" "$RED" "$NC"
                    exit 1
                fi
                INSTALL_DIR="$2"
                shift 2
                ;;
            --check)
                CHECK_ONLY=true
                shift
                ;;
            *)
                printf "%sError: Unknown option '%s'%s\n" "$RED" "$1" "$NC"
                echo "Run with --help for usage information."
                exit 1
                ;;
        esac
    done
}

print_help() {
    echo ""
    echo "claude-workbench installer"
    echo ""
    echo "USAGE:"
    echo "    bash install.sh [OPTIONS]"
    echo ""
    echo "OPTIONS:"
    echo "    -h, --help           Show this help message"
    echo "    --version            Show script version"
    echo "    --local              Build from source with cargo (requires Git repo checkout)"
    echo "    --install-dir DIR    Installation directory (default: ~/.local/bin)"
    echo "    --check              Only check dependencies, don't install"
    echo ""
    echo "EXAMPLES:"
    echo "    bash install.sh                              # Install latest release"
    echo "    bash install.sh --install-dir /usr/local/bin # Custom directory"
    echo "    bash install.sh --local                      # Build from source"
    echo "    bash install.sh --check                      # Check dependencies only"
    echo ""
    echo "ONE-LINER INSTALL:"
    printf "    curl -fsSL https://raw.githubusercontent.com/%s/main/scripts/install.sh | bash\n" "$REPO"
    echo ""
}

# --- UI Helpers ---

print_banner() {
    echo ""
    printf "%s╔════════════════════════════════════════════════════════════╗%s\n" "$BLUE" "$NC"
    printf "%s║%s        %sClaude Workbench%s — Installer v%s              %s║%s\n" "$BLUE" "$NC" "$BOLD" "$NC" "$SCRIPT_VERSION" "$BLUE" "$NC"
    printf "%s╚════════════════════════════════════════════════════════════╝%s\n" "$BLUE" "$NC"
    echo ""
}

print_row() {
    local content="$1"
    local len=${#content}
    local padding=$((56 - len))
    if (( padding < 0 )); then padding=0; fi
    printf "%s║%s  %s%*s%s║%s\n" "$BLUE" "$NC" "$content" "$padding" "" "$BLUE" "$NC"
}

print_step() {
    local step="$1"
    local total="$2"
    local msg="$3"
    printf "%s[%s/%s]%s %s\n" "$BLUE" "$step" "$total" "$NC" "$msg"
}

# --- Platform Detection ---

detect_platform() {
    local os arch asset

    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)
            case "$arch" in
                x86_64)  asset="${BINARY_NAME}-x86_64-unknown-linux-gnu.tar.gz" ;;
                aarch64) asset="${BINARY_NAME}-aarch64-unknown-linux-gnu.tar.gz" ;;
                *)
                    printf "%sError: Unsupported Linux architecture: %s%s\n" "$RED" "$arch" "$NC"
                    exit 1
                    ;;
            esac
            ;;
        Darwin)
            case "$arch" in
                arm64)   asset="${BINARY_NAME}-aarch64-apple-darwin.tar.gz" ;;
                x86_64)  asset="${BINARY_NAME}-x86_64-apple-darwin.tar.gz" ;;
                *)
                    printf "%sError: Unsupported macOS architecture: %s%s\n" "$RED" "$arch" "$NC"
                    exit 1
                    ;;
            esac
            ;;
        *)
            printf "%sError: Unsupported operating system: %s%s\n" "$RED" "$os" "$NC"
            echo "Use scripts/install.ps1 for Windows."
            exit 1
            ;;
    esac

    PLATFORM_OS="$os"
    PLATFORM_ARCH="$arch"
    ASSET_NAME="$asset"

    printf "  Platform:  %s%s %s%s\n" "$CYAN" "$os" "$arch" "$NC"
    printf "  Asset:     %s%s%s\n" "$DIM" "$asset" "$NC"
    echo ""
}

# --- Dependency Checking ---

check_dep() {
    local name="$1"
    local required="$2"
    local install_apt="${3:-}"
    local install_brew="${4:-}"
    local install_url="${5:-}"

    if command -v "$name" &>/dev/null; then
        local ver
        ver="$("$name" --version 2>/dev/null | head -1 || echo "installed")"
        printf "  %s✓%s %s %s(%s)%s\n" "$GREEN" "$NC" "$name" "$DIM" "$ver" "$NC"
        return 0
    else
        if [[ "$required" == "true" ]]; then
            printf "  %s✗%s %s %s(required)%s\n" "$RED" "$NC" "$name" "$RED" "$NC"
        else
            printf "  %s○%s %s %s(optional)%s\n" "$YELLOW" "$NC" "$name" "$DIM" "$NC"
        fi

        # Show install hint
        local hint=""
        if [[ "$PLATFORM_OS" == "Linux" && -n "$install_apt" ]]; then
            hint="$install_apt"
        elif [[ "$PLATFORM_OS" == "Darwin" && -n "$install_brew" ]]; then
            hint="$install_brew"
        elif [[ -n "$install_url" ]]; then
            hint="$install_url"
        fi

        if [[ -n "$hint" ]]; then
            printf "           %s→ %s%s\n" "$DIM" "$hint" "$NC"
        fi

        if [[ "$required" == "true" ]]; then
            return 1
        fi
        return 0
    fi
}

check_dependencies() {
    printf "%sDependency Check:%s\n" "$BOLD" "$NC"
    echo ""

    local has_errors=false

    # Required
    check_dep "git" "true" \
        "sudo apt install git" \
        "brew install git" \
        "https://git-scm.com" || has_errors=true

    # Optional
    check_dep "fish" "false" \
        "sudo apt install fish" \
        "brew install fish" \
        "https://fishshell.com"

    check_dep "lazygit" "false" \
        "https://github.com/jesseduffield/lazygit#installation" \
        "brew install lazygit" \
        "https://github.com/jesseduffield/lazygit"

    check_dep "claude" "false" \
        "https://docs.anthropic.com/en/docs/claude-code" \
        "https://docs.anthropic.com/en/docs/claude-code" \
        "https://docs.anthropic.com/en/docs/claude-code"

    # cargo only required for --local
    if [[ "$LOCAL_BUILD" == true ]]; then
        check_dep "cargo" "true" \
            "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" \
            "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" \
            "https://rustup.rs" || has_errors=true
    fi

    echo ""

    if [[ "$has_errors" == true ]]; then
        printf "%sMissing required dependencies. Please install them first.%s\n" "$RED" "$NC"
        return 1
    fi

    printf "%sAll required dependencies are available.%s\n" "$GREEN" "$NC"
    echo ""
    return 0
}

# --- Download & Install ---

download_release() {
    local url="${GITHUB_URL}/releases/latest/download/${ASSET_NAME}"
    local tmpdir

    tmpdir="$(mktemp -d)"
    TMPDIR_CLEANUP="$tmpdir"

    print_step 1 3 "Downloading latest release..."
    printf "  %s%s%s\n" "$DIM" "$url" "$NC"

    if command -v curl &>/dev/null; then
        curl -fsSL "$url" -o "${tmpdir}/${ASSET_NAME}"
    elif command -v wget &>/dev/null; then
        wget -q "$url" -O "${tmpdir}/${ASSET_NAME}"
    else
        printf "%sError: Neither curl nor wget found. Cannot download.%s\n" "$RED" "$NC"
        exit 1
    fi

    printf "  %s✓ Download complete%s\n" "$GREEN" "$NC"
    echo ""

    print_step 2 3 "Extracting archive..."
    tar xzf "${tmpdir}/${ASSET_NAME}" -C "$tmpdir"

    # Find the binary (may be in a subdirectory)
    local binary
    binary="$(find "$tmpdir" -name "$BINARY_NAME" -type f ! -name "*.tar.gz" | head -1)"

    if [[ -z "$binary" ]]; then
        printf "%sError: Binary not found in archive%s\n" "$RED" "$NC"
        exit 1
    fi

    printf "  %s✓ Extracted%s\n" "$GREEN" "$NC"
    echo ""

    install_binary "$binary"
}

build_local() {
    local project_dir

    # Try to find Cargo.toml relative to script or current dir
    if [[ -f "Cargo.toml" ]]; then
        project_dir="$(pwd)"
    elif [[ -f "$(dirname "$0")/../Cargo.toml" ]]; then
        project_dir="$(cd "$(dirname "$0")/.." && pwd)"
    else
        printf "%sError: Cargo.toml not found. Run --local from the project directory.%s\n" "$RED" "$NC"
        exit 1
    fi

    print_step 1 3 "Building release version..."
    printf "  %scargo build --release%s\n" "$DIM" "$NC"
    echo ""

    (cd "$project_dir" && cargo build --release 2>&1 | sed 's/^/      /')

    local binary="${project_dir}/target/release/${BINARY_NAME}"
    if [[ ! -f "$binary" ]]; then
        printf "%sError: Build failed — binary not found at %s%s\n" "$RED" "$binary" "$NC"
        exit 1
    fi

    echo ""
    printf "  %s✓ Build successful%s\n" "$GREEN" "$NC"
    echo ""

    install_binary "$binary"
}

install_binary() {
    local source="$1"

    print_step "$(( LOCAL_BUILD ? 2 : 3 ))" 3 "Installing binary..."

    # Create install directory
    if [[ ! -d "$INSTALL_DIR" ]]; then
        printf "  Creating directory: %s\n" "$INSTALL_DIR"
        mkdir -p "$INSTALL_DIR"
    fi

    cp "$source" "${INSTALL_DIR}/${BINARY_NAME}"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

    # macOS: remove quarantine attribute
    if [[ "$PLATFORM_OS" == "Darwin" ]]; then
        xattr -d com.apple.quarantine "${INSTALL_DIR}/${BINARY_NAME}" 2>/dev/null || true
    fi

    printf "  %s✓ Installed to %s/%s%s\n" "$GREEN" "$INSTALL_DIR" "$BINARY_NAME" "$NC"
    echo ""
}

# --- PATH Check ---

check_path() {
    if [[ ":${PATH}:" == *":${INSTALL_DIR}:"* ]]; then
        return
    fi

    printf "%sNote: %s is not in your PATH%s\n" "$YELLOW" "$INSTALL_DIR" "$NC"
    echo ""

    # Detect current shell for hint
    local shell_name
    shell_name="$(basename "${SHELL:-bash}")"

    case "$shell_name" in
        fish)
            printf "  Add to %s~/.config/fish/config.fish%s:\n" "$DIM" "$NC"
            printf "  %sfish_add_path %s%s\n" "$CYAN" "$INSTALL_DIR" "$NC"
            ;;
        zsh)
            printf "  Add to %s~/.zshrc%s:\n" "$DIM" "$NC"
            printf "  %sexport PATH=\"%s:\$PATH\"%s\n" "$CYAN" "$INSTALL_DIR" "$NC"
            ;;
        *)
            printf "  Add to %s~/.bashrc%s:\n" "$DIM" "$NC"
            printf "  %sexport PATH=\"%s:\$PATH\"%s\n" "$CYAN" "$INSTALL_DIR" "$NC"
            ;;
    esac
    echo ""
}

# --- Completion ---

print_completion() {
    local binary_path="${INSTALL_DIR}/${BINARY_NAME}"
    local binary_size version

    binary_size="$(ls -lh "$binary_path" | awk '{print $5}')"
    version="$("$binary_path" --version 2>/dev/null || echo "unknown")"

    printf "%s╔════════════════════════════════════════════════════════════╗%s\n" "$BLUE" "$NC"
    printf "%s║%s                  %sInstallation Complete%s                     %s║%s\n" "$BLUE" "$NC" "$GREEN" "$NC" "$BLUE" "$NC"
    printf "%s╠════════════════════════════════════════════════════════════╣%s\n" "$BLUE" "$NC"
    print_row "Binary:    ${BINARY_NAME}"
    print_row "Version:   ${version}"
    print_row "Size:      ${binary_size}"
    print_row "Location:  ${binary_path}"
    printf "%s╠════════════════════════════════════════════════════════════╣%s\n" "$BLUE" "$NC"
    printf "%s║%s  Run with:  %s%-45s%s%s║%s\n" "$BLUE" "$NC" "$YELLOW" "$BINARY_NAME" "$NC" "$BLUE" "$NC"
    printf "%s╚════════════════════════════════════════════════════════════╝%s\n" "$BLUE" "$NC"
    echo ""
}

# --- Main ---

main() {
    parse_args "$@"

    print_banner
    detect_platform

    if ! check_dependencies; then
        exit 1
    fi

    if [[ "$CHECK_ONLY" == true ]]; then
        echo "Dependency check complete. Use without --check to install."
        exit 0
    fi

    if [[ "$LOCAL_BUILD" == true ]]; then
        build_local
    else
        download_release
    fi

    check_path
    print_completion
}

main "$@"
