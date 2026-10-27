#!/usr/bin/env bash
set -euo pipefail

URL="https://github.com/hieudoanm/vectors/releases/download/latest/vectors"

ts() { date '+%Y-%m-%dT%H:%M:%S%z'; }

log()   { printf '[%s] [INFO ] %s\n' "$(ts)" "$*"; }
warn()  { printf '[%s] [WARN ] %s\n' "$(ts)" "$*" >&2; }
error() { printf '[%s] [ERROR] %s\n' "$(ts)" "$*" >&2; }
die()   { error "$*"; exit 1; }

# --- Detect the OS ---------------------------------------------------------
case "$(uname -s)" in
  Linux*)   OS=linux ;;
  Darwin*)  OS=macos ;;
  MINGW*|MSYS*|CYGWIN*) OS=windows ;;
  *)        die "Unsupported OS: $(uname -s)" ;;
esac
log "Detected OS: $OS"

# --- Pick a writable install destination -----------------------------------
# If the user passed an explicit path, trust it (after a sanity check).
if [ "$#" -ge 1 ] && [ -n "${1:-}" ]; then
  DEST="$1"
  log "Using user-specified destination: $DEST"
else
  # Prefer a user-local bin directory so non-root installs just work.
  DEST="$(command -v vectors 2>/dev/null || true)"
  if [ -n "$DEST" ]; then
    log "Found existing install at $DEST; will overwrite"
  elif [ "$OS" = "macos" ] && [ -d "/usr/local/bin" ] && [ -w "/usr/local/bin" ]; then
    DEST="/usr/local/bin/vectors"
  elif [ "$OS" = "linux" ] && [ -d "$HOME/.local/bin" ]; then
    mkdir -p "$HOME/.local/bin"
    DEST="$HOME/.local/bin/vectors"
  elif [ "$OS" = "linux" ] && [ -d "/usr/local/bin" ] && [ -w "/usr/local/bin" ]; then
    DEST="/usr/local/bin/vectors"
  elif [ "$OS" = "windows" ]; then
    DEST="$HOME/bin/vectors.exe"
    mkdir -p "$HOME/bin"
  else
    die "Could not find a writable install directory; pass one explicitly."
  fi
fi

DEST_DIR="$(dirname "$DEST")"
log "Preparing destination directory: $DEST_DIR"
mkdir -p "$DEST_DIR"
[ -w "$DEST_DIR" ] || die "$DEST_DIR is not writable"

# --- Download --------------------------------------------------------------
log "Downloading $URL"
tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT
curl -fsSL --retry 3 --retry-delay 2 "$URL" -o "$tmp"
log "Downloaded $(wc -c < "$tmp") bytes"

# --- Install ---------------------------------------------------------------
log "Installing to $DEST"
mv "$tmp" "$DEST"
chmod +x "$DEST"
log "Installed vectors to $DEST"

# --- Verify ----------------------------------------------------------------
if command -v "$DEST" >/dev/null 2>&1; then
  log "Verifying install: $DEST --version"
  "$DEST" --version || true
else
  log "Note: $DEST is not on PATH yet. Add $DEST_DIR to your shell's PATH."
fi
