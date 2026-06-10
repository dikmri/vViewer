#!/usr/bin/env sh
# vViewer installer for Linux and macOS
set -e

REPO="dikmri/vViewer"
RELEASES_URL="https://github.com/${REPO}/releases"
API_URL="https://api.github.com/repos/${REPO}/releases/latest"

OS="$(uname -s)"
ARCH="$(uname -m)"

get_latest_tag() {
  curl -sSfL "$API_URL" 2>/dev/null | grep '"tag_name"' | head -1 | cut -d'"' -f4
}

install_linux() {
  TAG="$(get_latest_tag)"
  case "$ARCH" in
    x86_64)  ASSET="vViewer_${TAG#v}_amd64.AppImage" ;;
    aarch64) ASSET="vViewer_${TAG#v}_aarch64.AppImage" ;;
    *)       echo "Unsupported architecture: $ARCH"; exit 1 ;;
  esac

  URL="${RELEASES_URL}/download/${TAG}/${ASSET}"
  DEST="${HOME}/.local/bin/vviewer"

  echo "Downloading vViewer ${TAG} for Linux/${ARCH}…"
  mkdir -p "$(dirname "$DEST")"
  curl -sSfL "$URL" -o "$DEST"
  chmod +x "$DEST"

  echo ""
  echo "✓ Installed to $DEST"
  echo "  Make sure ~/.local/bin is in your PATH."
}

install_macos() {
  TAG="$(get_latest_tag)"
  ASSET="vViewer_${TAG#v}_universal.dmg"
  URL="${RELEASES_URL}/download/${TAG}/${ASSET}"
  TMP="$(mktemp /tmp/vviewer_XXXXXX.dmg)"

  echo "Downloading vViewer ${TAG} for macOS…"
  curl -sSfL "$URL" -o "$TMP"

  echo "Installing…"
  VOLUME="$(hdiutil attach "$TMP" -nobrowse -quiet | awk 'END{print $NF}')"
  cp -r "${VOLUME}/vViewer.app" /Applications/
  hdiutil detach "$VOLUME" -quiet
  rm -f "$TMP"

  echo ""
  echo "✓ Installed to /Applications/vViewer.app"
  echo "  First launch: right-click → Open to bypass Gatekeeper."
}

case "$OS" in
  Linux*)  install_linux  ;;
  Darwin*) install_macos  ;;
  *)
    echo "Unsupported OS: $OS"
    echo "For Windows use:  irm https://raw.githubusercontent.com/${REPO}/main/install.ps1 | iex"
    exit 1
    ;;
esac
