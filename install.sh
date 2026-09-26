#!/usr/bin/env bash
# Install niri-tasks: build the binary, then link the four files that have to
# live outside this repo.
#
# Symlinks rather than copies, deliberately. The repo lives at a stable path, so
# a link means editing a file here is immediately live — there is no copy to
# keep in sync and so no snapshot-back step of the kind a dotfiles repo needs.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}"

GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
info() { echo -e "${GREEN}[+]${NC} $*"; }
warn() { echo -e "${YELLOW}[!]${NC} $*"; }

# Back up anything real that is in the way, then link. An existing symlink
# already pointing at us is left alone so re-runs are silent.
link() {
    local src="$1" dst="$2"
    if [ -L "$dst" ] && [ "$(readlink -f "$dst")" = "$(readlink -f "$src")" ]; then
        return 0
    fi
    mkdir -p "$(dirname "$dst")"
    if [ -e "$dst" ] || [ -L "$dst" ]; then
        local backup="$dst.before-niri-tasks.$(date +%Y%m%d-%H%M%S)"
        mv "$dst" "$backup"
        warn "Backed up existing: $dst → $backup"
    fi
    ln -s "$src" "$dst"
    info "Linked $dst"
}

echo "Installing niri-tasks from $REPO"

# ─── 1. the binary ────────────────────────────────────────────────────────────
if ! command -v cargo >/dev/null; then
    echo "cargo not found — install Rust first (https://rustup.rs)" >&2
    exit 1
fi
info "Building niritasks"
cargo install --path "$REPO" --locked

# The binary used to be called `wt`. cargo install does not remove a binary the
# package no longer builds, so a stale one would linger on PATH.
OLD_BIN="${CARGO_HOME:-$HOME/.cargo}/bin/wt"
if [ -x "$OLD_BIN" ]; then
    rm -f "$OLD_BIN"
    info "Removed the old wt binary"
fi

# ─── 2. the niri include ──────────────────────────────────────────────────────
# config.kdl must carry `include "niri-tasks.kdl"` for these binds to load. We do not
# edit config.kdl ourselves — it belongs to whoever owns the niri setup — so say
# so rather than silently doing nothing.
link "$REPO/niri/niri-tasks.kdl" "$CONFIG/niri/niri-tasks.kdl"

if [ -f "$CONFIG/niri/config.kdl" ] && ! grep -q 'include "niri-tasks.kdl"' "$CONFIG/niri/config.kdl"; then
    warn "$CONFIG/niri/config.kdl does not include niri-tasks.kdl — the keybinds will not load."
    warn "Add this line to it:"
    warn '    include "niri-tasks.kdl"'
fi

# ─── 3. the picker theme ──────────────────────────────────────────────────────
# Passed to fuzzel with --config=, so fuzzel.ini itself is left alone.
link "$REPO/fuzzel/picker.ini" "$CONFIG/fuzzel/picker.ini"

# ─── 4. the workspace-tasks skill ─────────────────────────────────────────────
# The skill Claude Code loads to work this list. It is carried in the repo
# because it is part of the tool, and linked for the same reason as everything
# else here: a copy is a thing to keep in sync, and this one was being synced by
# hand.
#
# The file, not the directory that holds it. Linking the directory would make
# `link` rename the existing one out of the way as `workspace-tasks.before-...`,
# which is still a skill directory with `name: workspace-tasks` inside it — two
# skills claiming one name. A displaced SKILL.md is inert.
CLAUDE_SKILLS="${CLAUDE_CONFIG_DIR:-$HOME/.claude}/skills"
link "$REPO/.claude/skills/workspace-tasks/SKILL.md" "$CLAUDE_SKILLS/workspace-tasks/SKILL.md"

# ─── 5. the overlay daemon ────────────────────────────────────────────────────
# A copy rather than a symlink: systemd reads unit files as root-ish early in
# session startup and does not follow links out of its search path reliably.
UNIT_DIR="$CONFIG/systemd/user"
mkdir -p "$UNIT_DIR"
if ! cmp -s "$REPO/systemd/niri-tasks.service" "$UNIT_DIR/niri-tasks.service"; then
    cp "$REPO/systemd/niri-tasks.service" "$UNIT_DIR/niri-tasks.service"
    info "Installed $UNIT_DIR/niri-tasks.service"
fi
systemctl --user daemon-reload 2>/dev/null || true

# `enable` then `restart`, not `enable --now`. --now only *starts* the unit, and
# does nothing at all when it is already running — so every update installed a
# new binary and left the old one on screen, until someone restarted it by hand
# or logged out. `restart` starts a stopped unit too, so it covers both.
systemctl --user enable niri-tasks.service 2>/dev/null \
    || warn "Could not enable niri-tasks.service — it will not start on login"

if systemctl --user restart niri-tasks.service 2>/dev/null; then
    info "Overlay daemon restarted on the new binary"
else
    warn "Could not start niri-tasks.service — start it with: systemctl --user start niri-tasks"
fi

# ─── 6. reload niri ───────────────────────────────────────────────────────────
if command -v niri >/dev/null && [ -n "${NIRI_SOCKET:-}" ]; then
    niri msg action load-config-file >/dev/null 2>&1 && info "Reloaded niri config" || true
fi

# ─── 7. dependencies ──────────────────────────────────────────────────────────
missing=()
for dep in niri task fuzzel tmux; do
    command -v "$dep" >/dev/null || missing+=("$dep")
done
if [ "${#missing[@]}" -gt 0 ]; then
    warn "Missing at runtime: ${missing[*]}"
fi

echo
info "Done. Keybinds: Mod+Alt+T add, Mod+Alt+Ctrl+T list, Mod+Alt+P project, Mod+Alt+W new workspace, Mod+Alt+Ctrl+W rename it."
