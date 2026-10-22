#!/usr/bin/env bash
# Checks the documented installs from a clean HOME, with no build step:
# Vim through vim-plug, and Helix through HELIX_RUNTIME. Needs git, curl,
# vim, hx and tmux. Usage: tests/native-install.sh [commit-or-branch]
set -euo pipefail
ref=${1:-main}
url=${HX_HIGHLIGHT_URL:-https://github.com/davidawad/hx-highlight}
work=$(mktemp -d)
export HOME=$work/home XDG_CONFIG_HOME=$work/home/.config XDG_DATA_HOME=$work/home/.local/share
mkdir -p "$HOME"
fail() {
  echo "native-install: FAIL: $*" >&2
  exit 1
}

# Vim: vim-plug fetches the repository; nothing is compiled.
curl -fsSLo "$HOME/.vim/autoload/plug.vim" --create-dirs \
  https://raw.githubusercontent.com/junegunn/vim-plug/master/plug.vim
key=branch
[[ $ref =~ ^[0-9a-f]{7,40}$ ]] && key=commit
cat > "$HOME/.vimrc" << EOF
call plug#begin()
Plug '$url', { '$key': '$ref' }
call plug#end()
EOF
vim -Es -u "$HOME/.vimrc" -i NONE -c PlugInstall -c qa || true
[ -f "$HOME/.vim/plugged/hx-highlight/plugin/hx-highlight.vim" ] || fail "vim-plug did not install plugin/hx-highlight.vim"
printf 'TODO fix this BEAD later DONE\n' > "$work/sample.txt"
vim -Es -u "$HOME/.vimrc" -i NONE -c "edit $work/sample.txt" \
  -c "if len(getmatches()) < 3 | cquit 3 | endif" \
  -c "if !hlexists('HxHighlight_todo') || !hlexists('HxHighlight_done') | cquit 4 | endif" \
  -c 'qa!' || fail "vim did not apply the markers (exit $?)"
echo "native-install: vim-plug OK"

# Helix: a plain clone on HELIX_RUNTIME; the theme and queries load from it.
git clone -q "$url" "$XDG_DATA_HOME/hx-highlight"
git -C "$XDG_DATA_HOME/hx-highlight" checkout -q "$ref"
export HELIX_RUNTIME=$XDG_DATA_HOME/hx-highlight/runtime
mkdir -p "$XDG_CONFIG_HOME/helix"
printf 'theme = "hx-highlight"\n' > "$XDG_CONFIG_HOME/helix/config.toml"
hx --health | grep -qF "$HELIX_RUNTIME" || fail "hx --health does not list $HELIX_RUNTIME"
printf '// TODO\n// BEAD\n// NOTE\nfn main() { let answer = true; }\n' > "$work/sample.rs"
sock=$work/tmux.sock
tmux -S "$sock" -f /dev/null new-session -d -x 100 -y 10 \
  "env TERM=xterm-256color COLORTERM=truecolor hx $work/sample.rs"
sleep 3
screen=$(tmux -S "$sock" capture-pane -e -p)
tmux -S "$sock" kill-server || true
for marker in TODO BEAD NOTE; do
  [[ $screen == *$'\e[38;5;15m\e[48;5;1m'"$marker"* ]] || fail "$marker is not white on red in hx:
$screen"
done
echo "native-install: helix OK ($(hx --version))"
