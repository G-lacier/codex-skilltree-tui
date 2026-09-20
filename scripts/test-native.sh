#!/usr/bin/env bash
set -euo pipefail
skilltree_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$skilltree_root/native"
exec cargo test "$@"
