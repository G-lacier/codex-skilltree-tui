# Native Codex integration

The editor compiles into Codex's Rust terminal interface. Supply an existing
Codex checkout to the build script:

```bash
./scripts/build-native.sh /path/to/codex
```

The script applies `native/codex.patch`, copies the editor modules into the TUI,
and builds both executables with Cargo. Each project uses its existing toolchain
and Cargo configuration. Additional command-line arguments are forwarded to
`cargo build`.

Cargo's JSON artifact messages provide the executable locations for the chosen
profile and target directory. The script uses jq to read those paths and copies
the programs into `SKILLTREE_BIN_DIR`, which defaults to `.skilltree/bin/`.

## Source layout

1. `native/src/catalog.rs`, `scan.rs`, and `storage.rs` implement the version 1
   catalog, metadata discovery, selective loading, and local configuration.
2. `editor.rs`, `form.rs`, and `render.rs` implement keyboard controls and Ratatui
   panels. Standalone behavior tests exercise the same code used inside Codex.
3. `codex.patch`, `codex-adapter.rs`, and `codex-chatwidget.rs` register the
   `/skilltree` command, mount a `BottomPaneView`, and return selected instructions
   through an application event to the current composer.

`/skilltree <task>` suggests up to three skills from their metadata. Ctrl+L expands
prerequisites, reads the selected files, checks their combined size, and inserts
the instructions and task into the composer. Enter submits the request through
the existing Codex conversation. Browsing and selection do not submit requests.

Selections enter the composer as ordinary user input with source and resource
paths. The catalog must be accessible on the same computer as the TUI; the editor
does not implement remote workspace file access.

The launcher sets `CODEX_SKILLTREE_ROOT` to this repository. Direct executable
invocation can use that variable to select another library. Otherwise the editor
locates `skill-trees.json` in the current directory or an ancestor.

The patch changes the TUI command registry and dispatch code. Dependency
resolution uses the selected checkout's manifests and Cargo lock file.

## Verification

Run the standalone behavior tests:

```bash
./scripts/test-native.sh
```

For tests compiled inside Codex, use that checkout's test runner for the
`codex-tui` package. With Cargo, `cargo test -p codex-tui skilltree` selects the
editor and command dispatch tests. The tests create self-contained generic
libraries using standard temporary directories.

Checks cover selective reads, prerequisite ordering and cycles, size budgets,
persistence, local instruction editing, keyboard assignment, Unicode text,
cancellation, terminal snapshots, and loading into the same composer without
starting a model request.

Reviewed snapshots in `native/src/snapshots/` use generic fixture data.

## Compatibility

The command uses internal Codex APIs. A different checkout may require changes
to the patch or adapter. Review `codex-rs/tui/src/slash_command.rs`, the dispatch
modules, the bottom-pane interface, and the composer insertion method in the
selected checkout, then rebuild and run the editor and command tests.
