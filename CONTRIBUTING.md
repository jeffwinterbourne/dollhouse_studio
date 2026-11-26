# Contributing to Dollhouse Studio

Thank you for your interest in contributing to **Dollhouse Studio**!

## Development Workflow

1. Fork the repository and create a feature branch from `main`.
2. Make sure code is cleanly modularized in the appropriate workspace crate (`dh-core`, `dh-geometry`, `dh-project`, `dh-export`, `dh-render`, or `dh-app`).
3. Add or update unit/integration tests for any geometry, export, project storage, or state logic changes. Geometry changes must preserve watertight 2-manifold meshes and zero unprintable overhangs.
4. Run the full verification suite locally before opening a pull request:
   ```sh
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   python scripts/hygiene_check.py
   ```

## Repository Hygiene

Please ensure commits do not contain:
- Local machine paths, user home directories, or IDE workspace settings
- Test photos, personal `.stl`/`.pdf` output files, or secrets/credentials
- Unrelated editor/tooling metadata files
