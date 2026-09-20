#!/usr/bin/env bash
# Integrate the editor into a supplied Codex checkout and build both executables.
set -euo pipefail
skilltree_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ $# -eq 0 ]]; then
    printf '%s\n' 'Usage: ./scripts/build-native.sh /path/to/codex [cargo build options]' >&2
    exit 1
fi
skilltree_source="$(cd -- "$1" && pwd)"
shift
skilltree_bin_dir="${SKILLTREE_BIN_DIR:-$skilltree_root/.skilltree/bin}"
mkdir -p "$skilltree_bin_dir"
skilltree_bin_dir="$(cd -- "$skilltree_bin_dir" && pwd)"

if git -C "$skilltree_source" apply --check "$skilltree_root/native/codex.patch" 2>/dev/null; then
    git -C "$skilltree_source" apply "$skilltree_root/native/codex.patch"
elif ! git -C "$skilltree_source" apply --reverse --check "$skilltree_root/native/codex.patch" 2>/dev/null; then
    printf '%s\n' 'The integration patch does not match this checkout. Review native/codex.patch against the selected Codex source.' >&2
    exit 1
fi

mkdir -p "$skilltree_source/codex-rs/tui/src/skilltree/snapshots"
install -m 644 "$skilltree_root"/native/src/*.rs "$skilltree_source/codex-rs/tui/src/skilltree/"
install -m 644 "$skilltree_root"/native/src/snapshots/*.snap "$skilltree_source/codex-rs/tui/src/skilltree/snapshots/"
for skilltree_mapping in 'codex-adapter.rs:skilltree_view.rs' 'codex-chatwidget.rs:chatwidget/skilltree.rs' 'codex-integration-tests.rs:chatwidget/tests/skilltree_native.rs'; do
    skilltree_from="${skilltree_mapping%%:*}"
    skilltree_to="$skilltree_source/codex-rs/tui/src/${skilltree_mapping#*:}"
    install -m 644 "$skilltree_root/native/$skilltree_from" "$skilltree_to"
done

# Cargo reports the actual artifact path, including custom profiles and targets.
skilltree_codex_binary="$(
    cd "$skilltree_source/codex-rs"
    cargo build -p codex-cli --bin codex "$@" --message-format=json-render-diagnostics |
        jq -r 'select(.reason == "compiler-artifact" and .target.name == "codex" and .executable != null) | .executable'
)"
install -m 755 "$skilltree_codex_binary" "$skilltree_bin_dir/codex-skilltree"
skilltree_cli_binary="$(
    cd "$skilltree_root/native"
    cargo build --bin codex-skilltree "$@" --message-format=json-render-diagnostics |
        jq -r 'select(.reason == "compiler-artifact" and .target.name == "codex-skilltree" and .executable != null) | .executable'
)"
install -m 755 "$skilltree_cli_binary" "$skilltree_bin_dir/skilltree"
printf '%s\n' 'Built. Run ./codex-skilltree, then type /skilltree inside Codex.'
