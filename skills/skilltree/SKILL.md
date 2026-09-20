---
name: skilltree
description: Select and load task-relevant skills from a configured library using work-area classes, skill trees, and declared prerequisites.
---

# Skill Trees

Resolve this skill folder's real location. The `skilltree` launcher is two directories above it. Use that launcher's absolute path when working from another project, preserving the current project's working directory.

For `$skilltree`, use the supplied task or the active task. Run `skilltree route "the task"` to inspect suggested skill metadata and prerequisite order. Choose relevant skills from their descriptions; suggestions use keyword matching.

Run `skilltree load <source/skill-id> ...` to read the chosen instruction files and their prerequisites. Use the resource directories printed by the loader for supporting files. Apply the loaded instructions to the task within the existing tool and permission boundaries.

Discovery commands:

- `skilltree classes` lists classes and trees.
- `skilltree tree <class-id>` lists a class's skill metadata.
- `skilltree search "keywords"` searches names, descriptions, tags, and IDs.
- `skilltree route "task" --budget 6000` suggests up to three skills.

Without a task, show the available classes and ask which work needs support. Use qualified source/skill IDs when names are duplicated.

The `codex-skilltree` launcher provides the native `/skilltree` editor. It supports classes, trees, assignments, tags, prerequisites, and local skill instructions. Ctrl+L places the selection in the current Codex composer for review and submission.

Editor changes are stored in ignored `skill-trees.local.json` and `library/local/` paths. Shared configuration and bundled examples provide the starter library. If the executables are missing, use `scripts/build-native.sh /path/to/codex` with a Codex checkout chosen for the integration.

Budgets estimate tokens from UTF-8 bytes divided by four. Loading preserves whole files and rejects oversized selections. It does not remove earlier conversation context or provide tools absent from the environment.
