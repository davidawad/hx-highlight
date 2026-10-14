# hx-highlight

Make `TODO`, `NOTE`, `BEAD` and `DONE` markers impossible to miss in Helix and Vim, from one JSON file you edit once.

![TODO, NOTE and BEAD white on red and DONE white on green, in Helix and Vim](docs/showcase/markers-hx-vim.png)

A marker is a name, a regex, and two colors:

```json
[
  {"name":"TODO","regex":"\\bTODO\\b","fg":"white","bg":"red"},
  {"name":"DONE","regex":"\\bDONE\\b","fg":"white","bg":"green"}
]
```

`hx-highlight` reads that list and writes a Helix theme, per-language Helix highlight queries, and a Vim plugin file. The screenshot above is rendered by the real `hx` and `vim` from `markers/markers.json` and the files in `docs/showcase/sample/`.

## Install

Needs a Rust toolchain, plus Helix and/or Vim.

```sh
git clone https://github.com/davidawad/hx-highlight && cd hx-highlight
cargo install --path .
```

### Helix

```sh
# theme (inherits gruvbox) and the Vim file
hx-highlight generate markers/markers.json /tmp/hxhl
mkdir -p ~/.config/helix/themes && cp /tmp/hxhl/hx-highlight.toml ~/.config/helix/themes/
# set  theme = "hx-highlight"  in ~/.config/helix/config.toml

# marker queries for the comment and org grammars
runtime="$(hx --health | sed -n 's/^Runtime directories: //p' | tr ';' '\n' \
  | while read -r d; do [ -f "$d/queries/comment/highlights.scm" ] && echo "$d" && break; done)"
hx-highlight queries markers/markers.json "$runtime" ~/.config/helix/runtime/queries
```

Rerun the `queries` step whenever you upgrade Helix or edit the marker list. Helix uses a user `highlights.scm` instead of its bundled one rather than extending it, so the generated file is the bundled query copied verbatim from your installed Helix with the marker patterns appended. For that reason the queries are generated on your machine and not shipped in this repository.

Helix markers are painted in code comments (via the `comment` grammar) and in `.org` files. Other prose grammars are not covered.

### Vim

```sh
hx-highlight generate markers/markers.json /tmp/hxhl
mkdir -p ~/.vim && cp /tmp/hxhl/hx-highlight.vim ~/.vim/
echo 'source ~/.vim/hx-highlight.vim' >> ~/.vimrc
```

Vim applies the markers with `matchadd()` at buffer and window entry, so your syntax highlighting is left intact.

## Usage

```sh
hx-highlight generate <markers.json> <output-dir>     # theme + vim file
hx-highlight queries  <markers.json> <helix-runtime> <queries-dir>
```

Add a marker by adding one entry to `markers/markers.json` and rerunning both commands. Colors are terminal color names; the Vim file maps the common names to hex for GUI Vim.

## Tests

```sh
cargo test
```

The tests compare generated files to goldens, source the Vim file in a real `vim`, and render a sample through a real `hx` in a private tmux server to check the painted colors. The last two skip when `vim`, `hx` or `tmux` is missing. The screenshot corpus and its reproduction commands are in [docs/showcase.md](docs/showcase.md).

## License

MIT, see [LICENSE](LICENSE).
