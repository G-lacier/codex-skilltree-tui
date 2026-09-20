# Codex Skilltree TUI

A terminal editor for organizing skill libraries and loading selected instructions into Codex. Browse skills by work area, group them into trees, edit classifications, and create local skills with `/skilltree`.

The editor, catalog, and loader are written in Rust. They compile into a Codex source checkout that you select. The repository includes a reusable framework and four generic example skills.

## Build and launch

The build requires Git, Cargo and a suitable Rust compiler, jq, and the development tools required by your Codex checkout. Choose or obtain a Codex checkout, then pass its location to the build script:

```bash
./scripts/build-native.sh /path/to/codex
./codex-skilltree
```

The script applies the integration patch and copies the editor modules into the supplied checkout. Cargo uses the toolchain, dependency caches, target directories, and build settings configured for each project. Additional arguments are passed to `cargo build`, for example:

```bash
./scripts/build-native.sh /path/to/codex --release -j 4
```

The resulting executables are copied from the paths reported by Cargo to `.skilltree/bin/`. Set `SKILLTREE_BIN_DIR` for both the build and launch commands to choose another output folder.

The patch depends on Codex's internal TUI interfaces. Check that it applies to your chosen checkout; changes to those interfaces may require updating the integration.

Inside the supplied Codex build, enter:

```text
/skilltree
```

Include a task to suggest relevant skills:

```text
/skilltree debug a regression in the application
```

Select skills and press **Ctrl+L** to place their instructions and prerequisites in the current composer. Review the text, then press **Enter** to submit it in the same conversation.

To use the library while working on another project:

```bash
./codex-skilltree --cd /path/to/project
```

Use the supplied launcher for the native `/skilltree` command. See [native integration](docs/native-integration.md) for the patch layout and development workflow.

## Editor controls

Wide terminals show three columns: **Classes**, **Trees**, and **Skills**. Narrow terminals show the active column. The details area displays descriptions, tags, prerequisites, and estimated instruction size.

| Key | Action |
| --- | --- |
| Tab / Shift+Tab, left / right | Change column |
| Up / down, Page Up / Page Down, Home / End | Browse the active column |
| Space, Enter on a skill | Select or deselect a skill |
| Ctrl+L | Load the selection into the current Codex composer |
| `/` | Search skill names, descriptions, IDs, and tags |
| `n` | Create a class, tree, or local skill in the active column |
| `e` | Edit a class or tree, or a skill's tags and prerequisites |
| `i` | Edit a local skill's description and instructions |
| `a` | Assign selected skills to the highlighted class and tree |
| `x` | Clear the selection |
| `t` | Set the task |
| `r` | Suggest skills for the task |
| `b` | Set the instruction budget |
| `s` | Register a local source folder |
| F5 | Rescan the library |
| Esc | Cancel a form or search, or close the editor |

Forms use **Tab** to switch fields, **Ctrl+S** to save, and **Esc** to cancel. Instruction fields accept multiline text and pasted content. Arrow keys, Home, End, Backspace, and Delete edit text; Ctrl+U clears the current field.

To move skills, select them with Space, browse to the destination class and tree, and press `a`. Selections remain active while browsing.

## Starter library

Classes describe work areas. Trees group related tasks within a class. Prerequisites identify instructions that another skill depends on.

| Class | Tree | Example skill |
| --- | --- | --- |
| Software Development | Codebase analysis | `project-map` |
| Debugging | Bug diagnosis | `trace-bug` |
| Data Analysis | Dataset inspection | `csv-profile` |
| Documentation | Technical writing | `technical-brief` |
| Unassigned | Unclassified skills | Skills without a matching rule |

`trace-bug` requires `project-map`, demonstrating prerequisite loading. The starter library is self-contained.

## Local configuration

| Location | Purpose |
| --- | --- |
| `skill-trees.json` | Shared classes, sources, and classification rules |
| `library/bundled/` | Generic example skills |
| `skill-trees.local.json` | Ignored local sources, classes, tags, and assignments |
| `library/local/` | Ignored local skills and source links |
| `.skilltree/` | Default executable output folder |
| `target/` | Cargo's default build output folder |

Editor changes are saved in local configuration, including overrides to shared classes. New skill bodies live in `library/local/created/`, where the editor can also modify their instructions. Classifications, tags, and prerequisites for linked skills can be changed without modifying the source library.

To add a library, open `/skilltree`, press `s`, enter a source ID and its folder path, and save with Ctrl+S. Paths can be absolute or relative to this repository. Existing symlinks also work.

A `skill-trees.local.json` example:

```json
{
  "sources": [
    {"id": "example", "path": "/path/to/skills"}
  ],
  "rules": [
    {
      "id": "example/debug-helper",
      "class": "debugging",
      "tree": "debugging",
      "tags": ["debug", "regression"],
      "requires": ["bundled/project-map"]
    }
  ]
}
```

Local classes replace shared classes with the same ID. Local sources and rules are appended; later matching rules take precedence. Renaming a class or tree preserves its ID and assignments. Rules can match `source`, a qualified skill `id`, or `names`. Patterns support `*`, `?`, and bracket groups. Sources support `optional` and `exclude` fields.

The metadata reader supports plain, single-quoted, JSON-style double-quoted, folded, and literal name and description strings. It reports malformed headers and invalid prerequisites.

## Selective loading

Browsing and task matching read skill headers. Loading reads the selected instruction files and their prerequisites in dependency order. Each loaded skill includes source and resource paths so Codex can locate supporting files. Browsing does not run skill scripts or start a model request.

Task suggestions match words in names, tags, and descriptions, selecting up to three skills. Review suggestions before loading. Broad requests or synonyms may require a different query or direct selection.

Instruction budgets estimate tokens from UTF-8 bytes divided by four, with a maximum setting of 10,000. The loader checks the combined size and includes whole files. Choose fewer skills if a selection exceeds the budget. This estimate is not an exact model token count.

The editor adds chosen instructions to the current request. Existing conversation context, global skill discovery, available tools, and Codex permissions remain controlled by Codex.

## Command-line and agent access

The build also produces a Rust catalog utility:

```bash
./skilltree classes
./skilltree tree debugging
./skilltree search "csv"
./skilltree route "debug a regression"
./skilltree load bundled/trace-bug --budget 6000
```

Discovery commands return JSON. `load` returns the selected instructions. The included `$skilltree` skill uses this utility for task-based selection. The `/skilltree` command opens the terminal editor.

## Verification

```bash
./scripts/test-native.sh
```

The script runs `cargo test` using the current environment. Tests create their own temporary fixtures through the standard temporary-directory API. Cargo options can be passed to the script.

Behavior tests cover selective reads, prerequisites, budgets, persistence, local instruction editing, keyboard controls, Unicode text, and rendering. Tests compiled inside Codex also cover slash-command dispatch, cancellation, and loading into the same composer. Generic terminal snapshots are stored in [native/src/snapshots](native/src/snapshots).
