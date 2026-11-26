# Changelog

All notable changes to **Dollhouse Studio** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

### Added
- Clean modular Cargo workspace (`dh-core`, `dh-geometry`, `dh-project`, `dh-export`, `dh-render`, `dh-app`).
- Self-contained disk project folders supporting both Single-Piece and Multi-Wall House projects, relative photo asset storage, explicit Save (`Ctrl+S`), and silent `.autosave` crash recovery.
- Live Component Instancing across one or multiple walls with per-instance overrides (`wall_thickness`, `acetate_thickness`, `hinge_right`, flush wall edges) and `Make Unique`.
- Personal and Project-scoped reusable Window/Door Templates with visual thumbnails.
- Office-inspired tabbed Ribbon, Start/Project Hub, CAD Project Explorer, slim contextual Properties panel, Floating Quick-Action Bar, and OS-adaptive Light & Dark themes.
- Display unit switching (`mm`, `cm`, `inches`) while maintaining exact internal millimeter geometry.
- Mouse-first 2D Blueprint & Photo Studio with direct move, 8-handle stretch/scale, rotate ring, live 4-corner perspective pins with magnifier loupe, crop brackets, visual 2-point caliper calibration, and optional Photo Setup Wizard.
- Upgraded 3D CAD viewport with interactive ViewCube, 3D picking, and on-wall 3D move/resize gizmos.
- Standard-paper multi-page vector PDF exports (A4, US Letter, A3) with tiled 1:1 full-wall elevations, packed 1:1 acetate/cutout sheets (`Unique × Qty` or `All Instances`), and Project Booklets with BOM cover sheets.
- Cross-platform CI checks, public-repo hygiene gate, and Windows NSIS / Portable ZIP / macOS DMG / Linux AppImage release automation.
