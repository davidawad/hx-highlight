#!/usr/bin/env bash
# Reproduce every screenshot in docs/showcase.md with the real hx and vim,
# inside a private tmux server and a throwaway config dir. Needs: cargo, hx,
# vim, tmux, python3, rsvg-convert. Nothing outside a temp dir is touched.
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
here="$root/docs/showcase"
tmp="$(mktemp -d)"
sock="hxhl-shots-$$"
trap 'tmux -L "$sock" kill-server 2>/dev/null || true; rm -rf "$tmp"' EXIT

cargo build --release --examples --bins --manifest-path "$root/Cargo.toml" >&2
bin="${HX_HIGHLIGHT_BIN:-${CARGO_TARGET_DIR:-$root/target}/release/hx-highlight}"

runtime="$(hx --health | sed -n 's/^Runtime directories: //p' | tr ';' '\n' |
  while read -r d; do [ -f "$d/queries/comment/highlights.scm" ] && echo "$d" && break; done)"

cfg="$tmp/cfg/helix"
mkdir -p "$cfg/themes" "$tmp/gen"
"$bin" generate "$root/markers/markers.json" "$tmp/gen"
cp "$tmp/gen/hx-highlight.toml" "$cfg/themes/hx-highlight.toml"
printf 'theme = "hx-highlight"\n' > "$cfg/config.toml"
"$bin" queries "$root/markers/markers.json" "$runtime" "$cfg/runtime/queries"

# shot NAME COMMAND: run COMMAND in an 80x12 pane, save the coloured capture.
shot() {
  tmux -L "$sock" -f /dev/null new-session -d -x 80 -y 12 "$2"
  sleep 2
  tmux -L "$sock" capture-pane -e -p > "$tmp/$1.ansi"
  tmux -L "$sock" kill-server
}

hx_cmd() { echo "env XDG_CONFIG_HOME=$tmp/cfg TERM=xterm-256color hx $1"; }
vim_cmd() {
  echo "env TERM=xterm-256color vim -N -u NONE -i NONE -c 'set background=dark number' -c 'syntax on' -c 'source $tmp/gen/hx-highlight.vim' -c 'edit!' $1"
}

shot hx-rs "$(hx_cmd "$here/sample/sample.rs")"
shot vim-rs "$(vim_cmd "$here/sample/sample.rs")"
shot hx-org "$(hx_cmd "$here/sample/notes.org")"

a2s="${CARGO_TARGET_DIR:-$root/target}/release/examples/ansi2svg"
"$a2s" "$tmp/top.svg" "Helix=$tmp/hx-rs.ansi" "Vim=$tmp/vim-rs.ansi"
rsvg-convert "$tmp/top.svg" -o "$here/markers-hx-vim.png"
"$a2s" "$tmp/org.svg" "Helix, org notes=$tmp/hx-org.ansi"
rsvg-convert "$tmp/org.svg" -o "$here/markers-org.png"
echo "wrote $here/markers-hx-vim.png and $here/markers-org.png" >&2
