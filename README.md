# Dollhouse Studio

**Dollhouse Studio** is a native desktop CAD application written in Rust for designing and manufacturing 3D-printable dollhouse windows, hinged doors, and full wall elevations.

Every generated component prints **outside-down with zero unprintable overhangs**, indexes cleanly across the dollhouse wall thickness around cut-to-size glazing acetate, and verifies as a watertight 2-manifold mesh.

## Highlights

- **Project Folders (`Single Piece` to `Whole House`)**: Self-contained, portable folder projects with relative asset storage, explicit Save (`Ctrl+S`), atomic file writes, and silent `.autosave` crash recovery.
- **Live Component Instancing**: Promote any window or door to a shared Component and place linked instances across one or multiple walls. Edit the shared Component once to update every instance while keeping per-instance overrides for wall thickness, hinge side, and flush edges.
- **Reusable Templates**: Save custom parameterized window/door setups into your **Personal Library** (available across all projects) or into **Project Templates** with style-only or full-size insertion.
- **Mouse-First 2D Blueprint & Wall Photo Studio**:
  - Import a reference photo onto any wall via an optional **Photo Setup Wizard** or directly on the 2D canvas.
  - Move, scale, edge-stretch, straighten/rotate, crop, apply live **4-corner perspective rectification** (with zoomed cursor loupe), and calibrate scale with a **2-point visual caliper** and unit-aware preset chips — with no number typing required.
  - Drag wall edges directly to resize, snap openings with smart magnetic alignment guides, and add interactive architectural dimensions.
- **3D CAD Viewport**:
  - Interactive **2D Blueprint**, **3D Wall Render** (with ViewCube, raycast picking, 3D drag/resize gizmos, and projected photo texture), and **3D Piece Inspector** (*Exploded*, *In Wall*, *Print Bed*, *Exterior/Interior Halves*).
- **16 Architectural Styles & Overhang-Free Detail**:
  - 11 window styles (fixed or print-in-place cone-hinged opening sashes) and 5 hinged door styles with 6 selectable hardware sets, letter boxes, cat flaps, shutters, classical headers/pediments, flower boxes, V-grooves, and flush wall cutbacks.
- **Manufacturing & Standard-Paper Multi-Page PDFs**:
  - Binary **STL** export and auto-nested **build plate packing** (`180²`, `220²`, `256²`, `300² mm`).
  - **1:1 SVG** cutting templates.
  - **Standard-Paper Multi-Page PDFs** (A4, US Letter, A3): tiled 1:1 full-wall elevations with crosshairs & overlap strips, auto-packed 1:1 cutout/acetate sheets (`Unique × Qty` or `All Instances`), and full Project Booklets with a parts BOM cover sheet.

## Architecture

The workspace is structured into focused, testable crates:

| Crate | Responsibility |
|---|---|
| `dh-core` | Pure domain model, display units (`mm`/`cm`/`in`), component instancing, templates, undo history |
| `dh-geometry` | Procedural CAD kernel, pluggable component builders, wall punching, manifold & overhang validation |
| `dh-project` | Disk folder persistence, atomic writes, crash recovery, asset copying, template libraries, photo warp math |
| `dh-export` | Binary STL, plate packer, 1:1 SVG, standard-paper multi-page vector PDF engine |
| `dh-render` | OpenGL (`glow`) 3D viewport, animated orbit camera, ViewCube, picking, 3D gizmos |
| `dh-app` | OS-adaptive Light/Dark desktop UI, Office-style ribbon, CAD Explorer, contextual Inspector, Blueprint 2D |

## Building & Testing

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable, edition 2024)
- On Linux: OpenGL / X11 / Wayland / GTK3 dev packages for `eframe` and `rfd` (e.g. `libgtk-3-dev`, `libxkbcommon-dev`, `libgl1-mesa-dev`, `libwayland-dev`).

### Run

```sh
cargo run --release -p dh-app
```

### Verify

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python scripts/hygiene_check.py
```

## License

Licensed under the [MIT License](LICENSE).
