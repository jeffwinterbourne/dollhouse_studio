# Changelog

All notable changes to **Dollhouse Studio** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-26

### Added
- Clean modular Cargo workspace (`dh-core`, `dh-geometry`, `dh-project`, `dh-export`, `dh-render`, `dh-app`).
- Self-contained disk project folders supporting both Single-Piece and Multi-Wall House projects, relative photo asset storage, explicit Save (`Ctrl+S`), and silent `.autosave` crash recovery.
- Live Component Instancing across one or multiple walls with per-instance overrides (`wall_thickness`, `acetate_thickness`, `hinge_right`, flush wall edges) and `Make Unique`.
- Personal and Project-scoped reusable Window/Door Templates with visual gallery thumbnails and inline Inspector style selector thumbnails.
- Icon-first Office-inspired tabbed Ribbon (defaulting to Home), floating Project Hub over live workspace, skippable 4-step Getting Started walkthrough, collapsible floating Fusion-360-style Browser tree, and resizable/hideable left Visual Library Gallery & right Properties Inspector.
- Display unit switching (`mm`, `cm`, `inches`) while maintaining exact internal millimeter geometry, plus extended acetate/glazing thickness support (slider up to `5 mm`, typed values up to `20 mm`).
- Mouse-first 2D Blueprint & Photo Studio with direct move, 8-handle stretch/scale, rotate ring, rotation-aware 4-corner perspective pins with magnifier loupe, crop brackets, visual multi-line calibration with exact non-round length entry and independent H/V stretch, large pan/zoom Photo Setup Wizard, right-click blueprint context menus, architrave tolerance overlay toggles, and object-linked CAD dimension chains.
- Exterior-front 3D CAD viewport (`Front` / `-Y` facing exterior) with interactive ViewCube, 3D picking, on-wall 3D move/resize gizmos, and synchronized Wall 3D / Piece 3D hardware & panelling rendering.
- Fixed symmetric hinged door panelling oriented on the exterior face (plus optional **Panelling on Interior Side** toggle) with cutout-clipped V-grooves that remain intact alongside letterboxes, cat flaps, and handles.
- Bed-aware 2D MaxRects STL print-plate packing (`Combined` bed-fitted plates by default, splitting multi-plate automatically to fit `BedSizePreset`) plus `1 STL / Part` individual export, SVG 1:1 cutting templates, and standard-paper multi-page vector PDFs (A4, US Letter, A3) with Booklet BOM cover sheets.
- Cross-platform CI checks, public-repo hygiene gate, and Windows NSIS Setup / Portable ZIP / macOS DMG / Linux AppImage release automation.
