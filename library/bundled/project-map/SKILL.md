---
name: project-map
description: Identify the entry points, modules, and data flow relevant to a requested software change in an unfamiliar codebase.
---

# Project map

Use manifests and application entry points to locate the component responsible for the requested behavior. Trace one representative input through its parsing, processing, storage, and output stages.

Record relevant file paths and symbols, including callers and configuration that influence the behavior. Separate observed relationships from assumptions that still need verification.

Return a focused map of the affected components and their existing validation commands. Expand into adjacent modules when their contracts affect the requested change.
