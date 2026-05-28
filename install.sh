#!/bin/bash
set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Cleanup trap
cleanup() {
  local exit_code=$?
  if [ $exit_code -ne 0 ]; then
    echo -e "\n${RED}✗ Installation failed (exit code: $exit_code)${NC}"
    echo "  Check the error above and try again."
  fi
}
trap cleanup EXIT

print_step() {
  echo -e "\n${GREEN}[$1/5]${NC} $2"
}

print_success() {
  echo -e "      ${GREEN}✓${NC} $1"
}

print_warning() {
  echo -e "      ${YELLOW}⚠${NC} $1"
}

print_error() {
  echo -e "      ${RED}✗${NC} $1"
}

# ---------------------------------------------------------------------------
# Step 1: Check Rust toolchain
# ---------------------------------------------------------------------------
check_rust() {
  print_step 1 "Checking Rust toolchain..."

  if command -v cargo &>/dev/null; then
    local cargo_version
    cargo_version=$(cargo --version)
    print_success "cargo found ($cargo_version)"
    return
  fi

  echo -e "      ${YELLOW}Rust toolchain not found.${NC}"
  printf "      Install Rust via mise? [Y/n] "
  read -r answer

  if [[ -z "$answer" || "$answer" =~ ^[Yy]$ ]]; then
    echo "      Installing mise and Rust..."
    curl -fsSL https://mise.jdx.dev/install.sh | sh
    # Add mise to PATH for this session
    export PATH="$HOME/.local/bin:$PATH"
    mise use -g rust
    # Source cargo env
    if [ -f "$HOME/.cargo/env" ]; then
      # shellcheck disable=SC1091
      source "$HOME/.cargo/env"
    fi
  else
    print_error "Please install Rust manually: https://rustup.rs"
    exit 1
  fi

  # Verify cargo works
  if ! command -v cargo &>/dev/null; then
    print_error "cargo still not found after install. Please check your PATH."
    exit 1
  fi

  local cargo_version
  cargo_version=$(cargo --version)
  print_success "cargo found ($cargo_version)"
}

# ---------------------------------------------------------------------------
# Step 2: Build
# ---------------------------------------------------------------------------
build() {
  print_step 2 "Building apprunner..."

  cargo build --release

  if [ ! -f "target/release/apprunner" ]; then
    print_error "Build failed: target/release/apprunner not found"
    exit 1
  fi

  print_success "Build successful"
}

# ---------------------------------------------------------------------------
# Step 3: Install binary
# ---------------------------------------------------------------------------
install_binary() {
  print_step 3 "Installing binary..."

  echo "      Where would you like to install?"
  echo "      [1] /usr/local/bin (may require sudo)"
  echo "      [2] ~/.local/bin (user-local, no sudo needed)"
  printf "      Choice [2]: "
  read -r choice

  # Default to 2
  if [ -z "$choice" ]; then
    choice="2"
  fi

  case "$choice" in
    1)
      INSTALL_DIR="/usr/local/bin"
      sudo cp target/release/apprunner "$INSTALL_DIR/apprunner"
      INSTALL_PATH="$INSTALL_DIR/apprunner"
      ;;
    2)
      INSTALL_DIR="$HOME/.local/bin"
      mkdir -p "$INSTALL_DIR"
      cp target/release/apprunner "$INSTALL_DIR/apprunner"
      INSTALL_PATH="$INSTALL_DIR/apprunner"
      ;;
    *)
      print_error "Invalid choice: $choice"
      exit 1
      ;;
  esac

  print_success "Installed to $INSTALL_PATH"

  # Check if install location is in PATH
  if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
    print_warning "$INSTALL_DIR is not in your PATH"
    print_warning "Add it with: export PATH=\"$INSTALL_DIR:\$PATH\""
  fi
}

# ---------------------------------------------------------------------------
# Step 4: Install zsh completions
# ---------------------------------------------------------------------------
install_completions() {
  print_step 4 "Setting up zsh completions..."

  mkdir -p ~/.zfunc

  # Use the installed binary path directly
  "$INSTALL_PATH" completions zsh > ~/.zfunc/_apprunner

  print_success "Completions installed to ~/.zfunc/_apprunner"
}

# ---------------------------------------------------------------------------
# Step 5: Configure shell
# ---------------------------------------------------------------------------
configure_shell() {
  print_step 5 "Configuring shell..."

  local zshrc="$HOME/.zshrc"
  local changed=false

  # Create .zshrc if it doesn't exist
  if [ ! -f "$zshrc" ]; then
    touch "$zshrc"
  fi

  # Check for fpath
  if ! grep -qE 'fpath\+=~/.zfunc|fpath\+=\$HOME/.zfunc|fpath\+="?\$HOME/.zfunc"?|fpath\+="?~/\.zfunc"?' "$zshrc"; then
    echo "" >> "$zshrc"
    echo "# apprunner completions" >> "$zshrc"
    echo 'fpath+=~/.zfunc' >> "$zshrc"
    print_success "Added fpath to ~/.zshrc"
    changed=true
  else
    print_success "fpath already configured in ~/.zshrc"
  fi

  # Check for compinit
  if ! grep -qE 'autoload.*compinit.*&&.*compinit|autoload.*compinit' "$zshrc"; then
    echo 'autoload -Uz compinit && compinit' >> "$zshrc"
    print_success "Added compinit to ~/.zshrc"
    changed=true
  else
    print_success "compinit already configured in ~/.zshrc"
  fi

  if [ "$changed" = false ]; then
    print_success "~/.zshrc already configured, no changes needed"
  fi
}

# ---------------------------------------------------------------------------
# Success message
# ---------------------------------------------------------------------------
print_done() {
  local display_path="$INSTALL_PATH"
  # Use ~ shorthand for home directory paths
  display_path="${display_path/#$HOME/\~}"

  echo ""
  echo -e "${GREEN}✓ apprunner installed successfully!${NC}"
  echo ""
  echo "Binary:      $display_path"
  echo "Completions: ~/.zfunc/_apprunner"
  echo ""
  echo "Restart your shell or run: source ~/.zshrc"
  echo "Then start with: apprunner"
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
main() {
  INSTALL_PATH=""

  check_rust
  build
  install_binary
  install_completions
  configure_shell
  print_done
}

main
