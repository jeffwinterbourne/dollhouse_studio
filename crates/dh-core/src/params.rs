use serde::{Deserialize, Serialize};

/// Available architectural window and hinged door styles in the gallery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WindowStyle {
    GeorgianGrid,
    SashSplit,
    CasementSplit,
    CottageLattice,
    ArchedGeorgian,
    GothicPointed,
    CircularBullsEye,
    CraftsmanPrairie,
    PictureFixed,
    PalladianVenetian,
    HalfRoundLunette,
    DoorColonial4Panel,
    DoorGeorgian6Panel,
    DoorGlazedFrench,
    DoorArchedHeritage,
    DoorFanlightGeorgian,
}

impl WindowStyle {
    pub const ALL_WINDOWS: [WindowStyle; 11] = [
        WindowStyle::GeorgianGrid,
        WindowStyle::SashSplit,
        WindowStyle::CasementSplit,
        WindowStyle::CottageLattice,
        WindowStyle::ArchedGeorgian,
        WindowStyle::GothicPointed,
        WindowStyle::CircularBullsEye,
        WindowStyle::CraftsmanPrairie,
        WindowStyle::PictureFixed,
        WindowStyle::PalladianVenetian,
        WindowStyle::HalfRoundLunette,
    ];

    pub const ALL_DOORS: [WindowStyle; 5] = [
        WindowStyle::DoorColonial4Panel,
        WindowStyle::DoorGeorgian6Panel,
        WindowStyle::DoorGlazedFrench,
        WindowStyle::DoorArchedHeritage,
        WindowStyle::DoorFanlightGeorgian,
    ];

    pub const ALL: [WindowStyle; 16] = [
        WindowStyle::GeorgianGrid,
        WindowStyle::SashSplit,
        WindowStyle::CasementSplit,
        WindowStyle::CottageLattice,
        WindowStyle::ArchedGeorgian,
        WindowStyle::GothicPointed,
        WindowStyle::CircularBullsEye,
        WindowStyle::CraftsmanPrairie,
        WindowStyle::PictureFixed,
        WindowStyle::PalladianVenetian,
        WindowStyle::HalfRoundLunette,
        WindowStyle::DoorColonial4Panel,
        WindowStyle::DoorGeorgian6Panel,
        WindowStyle::DoorGlazedFrench,
        WindowStyle::DoorArchedHeritage,
        WindowStyle::DoorFanlightGeorgian,
    ];

    pub fn is_door(self) -> bool {
        matches!(
            self,
            WindowStyle::DoorColonial4Panel
                | WindowStyle::DoorGeorgian6Panel
                | WindowStyle::DoorGlazedFrench
                | WindowStyle::DoorArchedHeritage
                | WindowStyle::DoorFanlightGeorgian
        )
    }

    pub fn has_acetate_glazing(self) -> bool {
        !matches!(
            self,
            WindowStyle::DoorColonial4Panel | WindowStyle::DoorGeorgian6Panel
        )
    }

    pub fn name(self) -> &'static str {
        match self {
            WindowStyle::GeorgianGrid => "Georgian Grid",
            WindowStyle::SashSplit => "Sash Split Pane",
            WindowStyle::CasementSplit => "Casement Mullion",
            WindowStyle::CottageLattice => "Cottage Lattice",
            WindowStyle::ArchedGeorgian => "Arched Georgian",
            WindowStyle::GothicPointed => "Gothic Pointed",
            WindowStyle::CircularBullsEye => "Circular Bull's-Eye",
            WindowStyle::CraftsmanPrairie => "Craftsman Prairie",
            WindowStyle::PictureFixed => "Picture Fixed Pane",
            WindowStyle::PalladianVenetian => "Palladian Venetian",
            WindowStyle::HalfRoundLunette => "Half-Round Lunette",
            WindowStyle::DoorColonial4Panel => "4-Panel Hinged Door",
            WindowStyle::DoorGeorgian6Panel => "6-Panel Hinged Door",
            WindowStyle::DoorGlazedFrench => "Glazed French Door",
            WindowStyle::DoorArchedHeritage => "Arched Hinged Door",
            WindowStyle::DoorFanlightGeorgian => "Fanlight Georgian Door",
        }
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            WindowStyle::GeorgianGrid => "Classic multi-pane grid",
            WindowStyle::SashSplit => "Fixed meeting rail + panes",
            WindowStyle::CasementSplit => "Mullion bays + transom",
            WindowStyle::CottageLattice => "Diagonal diamond leaded bars",
            WindowStyle::ArchedGeorgian => "Semicircular arch + spokes",
            WindowStyle::GothicPointed => "Lancet arch + Y-tracery",
            WindowStyle::CircularBullsEye => "Round frame + radial spokes",
            WindowStyle::CraftsmanPrairie => "Arts & Crafts upper mullions",
            WindowStyle::PictureFixed => "Clean unobstructed glass",
            WindowStyle::PalladianVenetian => "3-bay arched center + sidelights",
            WindowStyle::HalfRoundLunette => "Semicircular attic fanlight",
            WindowStyle::DoorColonial4Panel => "4 V-grooved panels + handle",
            WindowStyle::DoorGeorgian6Panel => "6 V-grooved panels + handle",
            WindowStyle::DoorGlazedFrench => "Acetate glazing + hinged frame",
            WindowStyle::DoorArchedHeritage => "Arched frame + hinged door",
            WindowStyle::DoorFanlightGeorgian => "Glazed radial fanlight + panels",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            WindowStyle::GeorgianGrid => {
                "Traditional Georgian multi-pane window with symmetrical vertical and horizontal glazing bars."
            }
            WindowStyle::SashSplit => {
                "Non-opening sash-look window featuring a heavier central meeting rail and customizable upper/lower grids."
            }
            WindowStyle::CasementSplit => {
                "Fixed casement-style frame divided by sturdy vertical mullions and an optional upper transom bar."
            }
            WindowStyle::CottageLattice => {
                "Rustic cottage window with intersecting 45-degree diagonal lattice bars."
            }
            WindowStyle::ArchedGeorgian => {
                "Georgian window topped with a semicircular arch and optional radial fanlight spokes."
            }
            WindowStyle::GothicPointed => {
                "Pointed Gothic lancet arch window with vertical mullions and intersecting arch tracery."
            }
            WindowStyle::CircularBullsEye => {
                "Round bull's-eye window with customizable radial spokes and optional inner ring."
            }
            WindowStyle::CraftsmanPrairie => {
                "Arts & Crafts / Prairie window with a high transom rail and corner sidelight bars."
            }
            WindowStyle::PictureFixed => {
                "Minimalist fixed picture window with a crisp frame and clear unobstructed acetate pane."
            }
            WindowStyle::PalladianVenetian => {
                "Classical three-bay Venetian window featuring a semicircular center arch ring flanked by sidelights."
            }
            WindowStyle::HalfRoundLunette => {
                "Semicircular attic or pediment fanlight window with customizable radial spokes and optional inner ring."
            }
            WindowStyle::DoorColonial4Panel => {
                "Hinged 4-panel dollhouse door with two-half indexed frame, integrated pivot pins, and separately printable lever handle."
            }
            WindowStyle::DoorGeorgian6Panel => {
                "Classic 6-panel hinged dollhouse door with V-grooved panels, indexed frame, and separate lever handle."
            }
            WindowStyle::DoorGlazedFrench => {
                "Hinged glazed door leaf sandwiching cut-to-size acetate with glazing bars, indexed frame, and separate handle."
            }
            WindowStyle::DoorArchedHeritage => {
                "Arched-top hinged dollhouse door with V-grooved panel or glazing, indexed frame, and separate handle."
            }
            WindowStyle::DoorFanlightGeorgian => {
                "Hinged Georgian entry door with an upper radial fanlight glazed in acetate above V-grooved lower panels."
            }
        }
    }

    pub fn supports_sill(self) -> bool {
        !matches!(self, WindowStyle::CircularBullsEye)
    }

    pub fn supports_opening_sash(self) -> bool {
        matches!(
            self,
            WindowStyle::GeorgianGrid
                | WindowStyle::SashSplit
                | WindowStyle::CasementSplit
                | WindowStyle::CottageLattice
                | WindowStyle::CraftsmanPrairie
                | WindowStyle::PictureFixed
                | WindowStyle::PalladianVenetian
        )
    }

    pub fn uses_grid_bars(self) -> bool {
        matches!(
            self,
            WindowStyle::GeorgianGrid
                | WindowStyle::SashSplit
                | WindowStyle::CasementSplit
                | WindowStyle::ArchedGeorgian
                | WindowStyle::PalladianVenetian
                | WindowStyle::DoorGlazedFrench
        )
    }

    pub fn uses_radial_spokes(self) -> bool {
        matches!(
            self,
            WindowStyle::ArchedGeorgian
                | WindowStyle::CircularBullsEye
                | WindowStyle::HalfRoundLunette
                | WindowStyle::DoorFanlightGeorgian
        )
    }

    /// Applies recommended style-specific defaults while preserving wall/print tolerances.
    pub fn apply_style_defaults(self, params: &mut WindowParams) {
        params.style = self;
        match self {
            WindowStyle::GeorgianGrid => {
                params.opening_width = 58.0;
                params.opening_height = 84.0;
                params.bars_x = 2;
                params.bars_y = 3;
                params.sill_enabled = true;
            }
            WindowStyle::SashSplit => {
                params.opening_width = 58.0;
                params.opening_height = 88.0;
                params.bars_x = 2;
                params.bars_y = 1;
                params.split_ratio = 0.50;
                params.meeting_rail_width = 3.0;
                params.sill_enabled = true;
            }
            WindowStyle::CasementSplit => {
                params.opening_width = 76.0;
                params.opening_height = 72.0;
                params.bars_x = 2;
                params.bars_y = 1;
                params.split_ratio = 0.72;
                params.meeting_rail_width = 2.6;
                params.sill_enabled = true;
            }
            WindowStyle::CottageLattice => {
                params.opening_width = 56.0;
                params.opening_height = 76.0;
                params.diamond_pitch = 14.0;
                params.sill_enabled = true;
            }
            WindowStyle::ArchedGeorgian => {
                params.opening_width = 54.0;
                params.opening_height = 92.0;
                params.bars_x = 2;
                params.bars_y = 2;
                params.arch_spokes = 3;
                params.sill_enabled = true;
            }
            WindowStyle::GothicPointed => {
                params.opening_width = 52.0;
                params.opening_height = 96.0;
                params.bars_x = 1;
                params.bars_y = 1;
                params.sill_enabled = true;
            }
            WindowStyle::CircularBullsEye => {
                params.opening_width = 52.0;
                params.opening_height = 52.0;
                params.arch_spokes = 4;
                params.bars_x = 1;
                params.sill_enabled = false;
            }
            WindowStyle::CraftsmanPrairie => {
                params.opening_width = 64.0;
                params.opening_height = 80.0;
                params.bars_x = 2;
                params.bars_y = 0;
                params.split_ratio = 0.72;
                params.meeting_rail_width = 2.8;
                params.sill_enabled = true;
            }
            WindowStyle::PictureFixed => {
                params.opening_width = 72.0;
                params.opening_height = 64.0;
                params.bars_x = 0;
                params.bars_y = 0;
                params.sill_enabled = true;
            }
            WindowStyle::PalladianVenetian => {
                params.opening_width = 92.0;
                params.opening_height = 84.0;
                params.bars_x = 2;
                params.bars_y = 2;
                params.split_ratio = 0.68;
                params.meeting_rail_width = 2.6;
                params.sill_enabled = true;
            }
            WindowStyle::HalfRoundLunette => {
                params.opening_width = 64.0;
                params.opening_height = 34.0;
                params.arch_spokes = 3;
                params.bars_x = 1;
                params.sill_enabled = true;
            }
            WindowStyle::DoorColonial4Panel => {
                params.opening_width = 76.0;
                params.opening_height = 166.0;
                params.double_doors = false;
                params.sill_enabled = false;
            }
            WindowStyle::DoorGeorgian6Panel => {
                params.opening_width = 76.0;
                params.opening_height = 166.0;
                params.double_doors = false;
                params.sill_enabled = false;
            }
            WindowStyle::DoorGlazedFrench => {
                params.opening_width = 96.0;
                params.opening_height = 166.0;
                params.double_doors = true;
                params.bars_x = 1;
                params.bars_y = 2;
                params.lower_panel_ratio = 0.32;
                params.sill_enabled = false;
            }
            WindowStyle::DoorArchedHeritage => {
                params.opening_width = 76.0;
                params.opening_height = 172.0;
                params.double_doors = false;
                params.bars_x = 1;
                params.bars_y = 1;
                params.sill_enabled = false;
            }
            WindowStyle::DoorFanlightGeorgian => {
                params.opening_width = 78.0;
                params.opening_height = 178.0;
                params.double_doors = false;
                params.arch_spokes = 3;
                params.bars_x = 1;
                params.bars_y = 2;
                params.sill_enabled = false;
            }
        }
    }
}

/// Decorative V-groove fluting style cut into the architrave face (41° overhang-free V-channel).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum GrooveStyle {
    #[default]
    None,
    SingleVGroove,
    DoubleVGroove,
}

impl GrooveStyle {
    pub const ALL: [GrooveStyle; 3] = [
        GrooveStyle::None,
        GrooveStyle::SingleVGroove,
        GrooveStyle::DoubleVGroove,
    ];

    pub fn name(self) -> &'static str {
        match self {
            GrooveStyle::None => "No Grooves",
            GrooveStyle::SingleVGroove => "Single V-Groove",
            GrooveStyle::DoubleVGroove => "Double V-Grooves",
        }
    }
}

/// Optional overhang-free exterior shutters integrated on either side of the exterior architrave.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ShutterStyle {
    #[default]
    None,
    Louvred,
    Panelled,
}

impl ShutterStyle {
    pub const ALL: [ShutterStyle; 3] = [
        ShutterStyle::None,
        ShutterStyle::Louvred,
        ShutterStyle::Panelled,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ShutterStyle::None => "No Shutters",
            ShutterStyle::Louvred => "Louvred Shutters",
            ShutterStyle::Panelled => "Panelled Shutters",
        }
    }
}

/// Optional classical architectural header / pediment above rectangular exterior architraves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum HeaderStyle {
    #[default]
    None,
    FlatLintel,
    KeystoneArch,
    TriangularPediment,
}

impl HeaderStyle {
    pub const ALL: [HeaderStyle; 4] = [
        HeaderStyle::None,
        HeaderStyle::FlatLintel,
        HeaderStyle::KeystoneArch,
        HeaderStyle::TriangularPediment,
    ];

    pub fn name(self) -> &'static str {
        match self {
            HeaderStyle::None => "Standard Trim Top",
            HeaderStyle::FlatLintel => "Flat Cornice Lintel",
            HeaderStyle::KeystoneArch => "Keystone Header",
            HeaderStyle::TriangularPediment => "Triangular Pediment",
        }
    }
}

/// How the top exterior architrave handles pre-existing architectural features flush with the wall opening.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum FlushTopMode {
    #[default]
    None,
    HorizontalHead,
    FullContour,
}

impl FlushTopMode {
    pub const ALL: &'static [FlushTopMode] = &[
        FlushTopMode::None,
        FlushTopMode::HorizontalHead,
        FlushTopMode::FullContour,
    ];

    pub fn name(self) -> &'static str {
        match self {
            FlushTopMode::None => "Full Trim",
            FlushTopMode::HorizontalHead => "Flush Horizontal Head",
            FlushTopMode::FullContour => "Flush Contour / Arch",
        }
    }
}

/// Hardware handle styles for hinged dollhouse doors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum DoorHandleStyle {
    #[default]
    ModernLever,
    VictorianScroll,
    RoundKnob,
    RingLatch,
    PullBar,
    None,
}

impl DoorHandleStyle {
    pub const ALL: &'static [DoorHandleStyle] = &[
        DoorHandleStyle::ModernLever,
        DoorHandleStyle::VictorianScroll,
        DoorHandleStyle::RoundKnob,
        DoorHandleStyle::RingLatch,
        DoorHandleStyle::PullBar,
        DoorHandleStyle::None,
    ];

    pub fn name(self) -> &'static str {
        match self {
            DoorHandleStyle::ModernLever => "Modern Lever & Plate",
            DoorHandleStyle::VictorianScroll => "Victorian Scroll Lever",
            DoorHandleStyle::RoundKnob => "Heritage Round Knob",
            DoorHandleStyle::RingLatch => "Cottage Ring Latch",
            DoorHandleStyle::PullBar => "Contemporary Bar Pull",
            DoorHandleStyle::None => "No Handle",
        }
    }
}

/// Opening sash hinge styles (print-in-place cone-in-socket hinges) for rectangular windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum WindowOpeningMode {
    #[default]
    Fixed,
    CasementLeft,
    CasementRight,
    CasementDouble,
    AwningTop,
    HopperBottom,
}

impl WindowOpeningMode {
    pub const ALL: &'static [WindowOpeningMode] = &[
        WindowOpeningMode::Fixed,
        WindowOpeningMode::CasementLeft,
        WindowOpeningMode::CasementRight,
        WindowOpeningMode::CasementDouble,
        WindowOpeningMode::AwningTop,
        WindowOpeningMode::HopperBottom,
    ];

    pub fn name(self) -> &'static str {
        match self {
            WindowOpeningMode::Fixed => "Fixed (Non-Opening)",
            WindowOpeningMode::CasementLeft => "Casement — Left Hinge",
            WindowOpeningMode::CasementRight => "Casement — Right Hinge",
            WindowOpeningMode::CasementDouble => "Double French Casements",
            WindowOpeningMode::AwningTop => "Top-Hung Awning",
            WindowOpeningMode::HopperBottom => "Bottom-Hung Hopper",
        }
    }
}

/// 1-click printer tolerance presets for Fit & Print tuning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PrinterPreset {
    #[default]
    Standard04,
    Fine02,
    Resin,
    LooseFit,
}

impl PrinterPreset {
    pub const ALL: [PrinterPreset; 4] = [
        PrinterPreset::Standard04,
        PrinterPreset::Fine02,
        PrinterPreset::Resin,
        PrinterPreset::LooseFit,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PrinterPreset::Standard04 => "0.4mm FDM",
            PrinterPreset::Fine02 => "0.2mm Fine",
            PrinterPreset::Resin => "SLA Resin",
            PrinterPreset::LooseFit => "Loose Fit",
        }
    }

    pub fn apply(self, p: &mut WindowParams) {
        match self {
            PrinterPreset::Standard04 => {
                p.acetate_clearance = 0.25;
                p.wall_fit_clearance = 0.25;
                p.index_clearance = 0.15;
                p.hinge_clearance = 0.35;
            }
            PrinterPreset::Fine02 => {
                p.acetate_clearance = 0.15;
                p.wall_fit_clearance = 0.15;
                p.index_clearance = 0.10;
                p.hinge_clearance = 0.25;
            }
            PrinterPreset::Resin => {
                p.acetate_clearance = 0.12;
                p.wall_fit_clearance = 0.12;
                p.index_clearance = 0.08;
                p.hinge_clearance = 0.22;
            }
            PrinterPreset::LooseFit => {
                p.acetate_clearance = 0.40;
                p.wall_fit_clearance = 0.40;
                p.index_clearance = 0.25;
                p.hinge_clearance = 0.48;
            }
        }
    }
}

/// Dollhouse scale presets for quick sizing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ScalePreset {
    #[default]
    Scale1_12,
    Scale1_24,
    Scale1_48,
}

impl ScalePreset {
    pub const ALL: [ScalePreset; 3] = [
        ScalePreset::Scale1_12,
        ScalePreset::Scale1_24,
        ScalePreset::Scale1_48,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ScalePreset::Scale1_12 => "1:12 Standard",
            ScalePreset::Scale1_24 => "1:24 Half",
            ScalePreset::Scale1_48 => "1:48 Quarter",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            ScalePreset::Scale1_12 => "1:12",
            ScalePreset::Scale1_24 => "1:24",
            ScalePreset::Scale1_48 => "1:48",
        }
    }

    /// Factor relative to 1:12 base dimensions.
    pub fn scale_factor(self) -> f64 {
        match self {
            ScalePreset::Scale1_12 => 1.0,
            ScalePreset::Scale1_24 => 0.5,
            ScalePreset::Scale1_48 => 0.25,
        }
    }
}

/// Architectural molding profile for the exterior and interior architrave trim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ArchitraveStyle {
    Flat,
    #[default]
    Chamfered,
    Stepped,
    ChamferedStepped,
}

impl ArchitraveStyle {
    pub const ALL: [ArchitraveStyle; 4] = [
        ArchitraveStyle::Flat,
        ArchitraveStyle::Chamfered,
        ArchitraveStyle::Stepped,
        ArchitraveStyle::ChamferedStepped,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ArchitraveStyle::Flat => "Flat Square",
            ArchitraveStyle::Chamfered => "Chamfered Bevel",
            ArchitraveStyle::Stepped => "Stepped Molding",
            ArchitraveStyle::ChamferedStepped => "Chamfered + Stepped",
        }
    }

    pub fn has_chamfer(self) -> bool {
        matches!(
            self,
            ArchitraveStyle::Chamfered | ArchitraveStyle::ChamferedStepped
        )
    }

    pub fn has_step(self) -> bool {
        matches!(
            self,
            ArchitraveStyle::Stepped | ArchitraveStyle::ChamferedStepped
        )
    }
}

/// 3D piece viewport display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ViewMode {
    PrintPlate,
    #[default]
    Exploded,
    Assembled,
    ExteriorOnly,
    InteriorOnly,
}

impl ViewMode {
    pub const ALL: [ViewMode; 5] = [
        ViewMode::Exploded,
        ViewMode::Assembled,
        ViewMode::PrintPlate,
        ViewMode::ExteriorOnly,
        ViewMode::InteriorOnly,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ViewMode::PrintPlate => "Print Bed (Outside-Down)",
            ViewMode::Exploded => "Exploded Sandwich",
            ViewMode::Assembled => "Assembled in Wall",
            ViewMode::ExteriorOnly => "Exterior Half",
            ViewMode::InteriorOnly => "Interior Half",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            ViewMode::PrintPlate => "Print Bed",
            ViewMode::Exploded => "Exploded",
            ViewMode::Assembled => "In Wall",
            ViewMode::ExteriorOnly => "Ext Half",
            ViewMode::InteriorOnly => "Int Half",
        }
    }
}

/// Filament preview colors in the 3D viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FilamentColor {
    HeritageWhite,
    #[default]
    WarmCream,
    OakWoodfill,
    SageGreen,
    SlateGrey,
    HeritageRed,
    CharcoalBlack,
    TerracottaBrick,
}

impl FilamentColor {
    pub const ALL: [FilamentColor; 8] = [
        FilamentColor::HeritageWhite,
        FilamentColor::WarmCream,
        FilamentColor::OakWoodfill,
        FilamentColor::SageGreen,
        FilamentColor::SlateGrey,
        FilamentColor::HeritageRed,
        FilamentColor::CharcoalBlack,
        FilamentColor::TerracottaBrick,
    ];

    pub fn name(self) -> &'static str {
        match self {
            FilamentColor::HeritageWhite => "White",
            FilamentColor::WarmCream => "Cream",
            FilamentColor::OakWoodfill => "Oak",
            FilamentColor::SageGreen => "Sage",
            FilamentColor::SlateGrey => "Slate",
            FilamentColor::HeritageRed => "Heritage Red",
            FilamentColor::CharcoalBlack => "Charcoal",
            FilamentColor::TerracottaBrick => "Terracotta",
        }
    }

    pub fn rgb_exterior(self) -> [f32; 3] {
        match self {
            FilamentColor::HeritageWhite => [0.93, 0.93, 0.91],
            FilamentColor::WarmCream => [0.95, 0.90, 0.79],
            FilamentColor::OakWoodfill => [0.68, 0.50, 0.33],
            FilamentColor::SageGreen => [0.56, 0.67, 0.57],
            FilamentColor::SlateGrey => [0.43, 0.48, 0.54],
            FilamentColor::HeritageRed => [0.64, 0.25, 0.23],
            FilamentColor::CharcoalBlack => [0.20, 0.22, 0.25],
            FilamentColor::TerracottaBrick => [0.72, 0.36, 0.26],
        }
    }

    pub fn rgb_interior(self) -> [f32; 3] {
        let e = self.rgb_exterior();
        [
            (e[0] * 0.92 + 0.06).min(1.0),
            (e[1] * 0.92 + 0.05).min(1.0),
            (e[2] * 0.92 + 0.04).min(1.0),
        ]
    }
}

/// Full parametric specification for a 3D-printable two-half dollhouse window or hinged door.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowParams {
    pub style: WindowStyle,
    /// Width of the wall cutout opening (mm).
    pub opening_width: f64,
    /// Height of the wall cutout opening (mm).
    pub opening_height: f64,
    /// Thickness of the dollhouse wall (mm).
    pub wall_thickness: f64,

    /// Exterior architrave flange width beyond the wall opening (mm).
    pub trim_width: f64,
    /// Exterior architrave thickness proud of the wall (mm).
    pub trim_thickness: f64,
    /// Interior architrave flange width beyond the wall opening (mm).
    pub interior_trim_width: f64,
    /// Interior architrave thickness proud of the wall (mm).
    pub interior_trim_thickness: f64,
    /// Link interior trim width/depth to exterior trim dimensions in the UI.
    #[serde(default = "default_true")]
    pub match_interior_trim: bool,
    /// Architrave molding profile.
    #[serde(default)]
    pub architrave_style: ArchitraveStyle,
    /// Chamfer bevel width on the architrave face (mm).
    #[serde(default = "default_chamfer_size")]
    pub chamfer_size: f64,
    /// Whether to apply the chamfer/step molding to the interior trim as well.
    #[serde(default = "default_true")]
    pub chamfer_interior: bool,
    /// Apply 41-degree chamfers to glazing bars and inner frame openings.
    #[serde(default = "default_true")]
    pub chamfer_bars: bool,
    /// Bevel width on each side of glazing bars and inner pane edges (mm).
    #[serde(default = "default_bar_chamfer")]
    pub bar_chamfer_size: f64,
    /// Decorative V-groove fluting on the architrave face.
    #[serde(default)]
    pub groove_style: GrooveStyle,
    /// Width of each V-groove channel on the front face (mm).
    #[serde(default = "default_groove_width")]
    pub groove_width: f64,
    /// Optional exterior shutters integrated with the exterior architrave.
    #[serde(default)]
    pub shutter_style: ShutterStyle,
    /// Width of each shutter panel as a fraction of opening width (`0.22..0.55`).
    #[serde(default = "default_shutter_width_ratio")]
    pub shutter_width_ratio: f64,
    /// Optional classical header / pediment above rectangular exterior architraves.
    #[serde(default)]
    pub header_style: HeaderStyle,
    /// Optional planter/flower box attached beneath the exterior sill.
    #[serde(default)]
    pub flower_box_enabled: bool,
    /// Pre-existing top feature flush with the opening.
    #[serde(default)]
    pub flush_top_mode: FlushTopMode,
    /// Pre-existing sill flush with the bottom of the wall opening.
    #[serde(default)]
    pub flush_bottom_sill: bool,
    /// Pre-existing feature flush with the left jamb edge.
    #[serde(default)]
    pub flush_left_jamb: bool,
    /// Pre-existing feature flush with the right jamb edge.
    #[serde(default)]
    pub flush_right_jamb: bool,
    /// Extra trim setback (mm) behind flush opening edges.
    #[serde(default)]
    pub flush_trim_setback: f64,

    /// Hinged door options: double French doors vs single door.
    #[serde(default)]
    pub double_doors: bool,
    /// Hinge side for single doors (false = left, true = right).
    #[serde(default)]
    pub hinge_right: bool,
    /// Thickness of the hinged door leaf (mm).
    #[serde(default = "default_door_thickness")]
    pub door_thickness: f64,
    /// Rotational clearance around the door leaf and hinge pins (mm).
    #[serde(default = "default_hinge_clearance")]
    pub hinge_clearance: f64,
    /// Solid lower kick-panel height ratio for glazed doors (`0.0..0.6`).
    #[serde(default = "default_lower_panel_ratio")]
    pub lower_panel_ratio: f64,
    /// Selected handle hardware style for hinged doors.
    #[serde(default)]
    pub door_handle_style: DoorHandleStyle,
    /// Whether to cut a brass-plated mail letter box slot into the door.
    #[serde(default)]
    pub letterbox_enabled: bool,
    /// Normalized height of the letter box up the door leaf (`0.22..0.72`).
    #[serde(default = "default_letterbox_height")]
    pub letterbox_height_ratio: f64,
    /// Whether to cut a lower hinged cat/pet flap opening into the door.
    #[serde(default)]
    pub cat_flap_enabled: bool,
    /// Clear opening width of the cat flap (mm).
    #[serde(default = "default_cat_flap_w")]
    pub cat_flap_width: f64,
    /// Clear opening height of the cat flap (mm).
    #[serde(default = "default_cat_flap_h")]
    pub cat_flap_height: f64,
    /// Optional print-in-place cone-hinged opening sash mode for rectangular windows.
    #[serde(default)]
    pub window_opening_mode: WindowOpeningMode,

    /// Whether to include a projecting exterior window sill at the bottom.
    pub sill_enabled: bool,
    /// Lateral overhang of the sill beyond the architrave sides (mm).
    pub sill_overhang: f64,
    /// Downward apron depth of the sill below the bottom architrave (mm).
    pub sill_depth: f64,

    /// Total width of the inner frame + barrel wall inside the cutout (mm).
    pub frame_width: f64,
    /// Vertical glazing bar count (or bays / inner-ring toggle for circular).
    pub bars_x: u32,
    /// Horizontal glazing bar count.
    pub bars_y: u32,
    /// Width of standard glazing bars (mm).
    pub bar_width: f64,
    /// Width of the thicker meeting/transom rail (mm).
    pub meeting_rail_width: f64,
    /// Vertical split fraction (`0.25..0.85`).
    pub split_ratio: f64,
    /// Number of radial spokes in Arched / Circular / Lunette / Fanlight styles.
    pub arch_spokes: u32,
    /// Diagonal lattice spacing pitch in CottageLattice (mm).
    pub diamond_pitch: f64,

    /// Thickness of the cut-to-size acetate sheet (mm).
    pub acetate_thickness: f64,
    /// Radial clearance around the acetate inside the pocket (mm).
    pub acetate_clearance: f64,
    /// Clearance between outer barrel and dollhouse wall cutout per side (mm).
    pub wall_fit_clearance: f64,
    /// Radial clearance for telescopic indexing between exterior barrel and interior spigot (mm).
    pub index_clearance: f64,
    /// Gap between halves when arranged on the print bed (mm).
    pub plate_spacing: f64,
}

fn default_chamfer_size() -> f64 {
    0.9
}
fn default_true() -> bool {
    true
}
fn default_bar_chamfer() -> f64 {
    0.30
}
fn default_groove_width() -> f64 {
    0.85
}
fn default_door_thickness() -> f64 {
    4.0
}
fn default_hinge_clearance() -> f64 {
    0.35
}
fn default_lower_panel_ratio() -> f64 {
    0.32
}
fn default_shutter_width_ratio() -> f64 {
    0.38
}
fn default_letterbox_height() -> f64 {
    0.48
}
fn default_cat_flap_w() -> f64 {
    14.0
}
fn default_cat_flap_h() -> f64 {
    15.0
}

impl Default for WindowParams {
    fn default() -> Self {
        Self {
            style: WindowStyle::GeorgianGrid,
            opening_width: 58.0,
            opening_height: 84.0,
            wall_thickness: 9.0,
            trim_width: 5.5,
            trim_thickness: 2.2,
            interior_trim_width: 5.0,
            interior_trim_thickness: 1.8,
            match_interior_trim: true,
            architrave_style: ArchitraveStyle::Chamfered,
            chamfer_size: 0.9,
            chamfer_interior: true,
            chamfer_bars: true,
            bar_chamfer_size: 0.30,
            groove_style: GrooveStyle::SingleVGroove,
            groove_width: 0.85,
            shutter_style: ShutterStyle::None,
            shutter_width_ratio: 0.38,
            header_style: HeaderStyle::None,
            flower_box_enabled: false,
            flush_top_mode: FlushTopMode::None,
            flush_bottom_sill: false,
            flush_left_jamb: false,
            flush_right_jamb: false,
            flush_trim_setback: 0.0,
            double_doors: false,
            hinge_right: false,
            door_thickness: 4.0,
            hinge_clearance: 0.35,
            lower_panel_ratio: 0.32,
            door_handle_style: DoorHandleStyle::ModernLever,
            letterbox_enabled: false,
            letterbox_height_ratio: 0.48,
            cat_flap_enabled: false,
            cat_flap_width: 14.0,
            cat_flap_height: 15.0,
            window_opening_mode: WindowOpeningMode::Fixed,
            sill_enabled: true,
            sill_overhang: 3.5,
            sill_depth: 3.0,
            frame_width: 5.0,
            bars_x: 2,
            bars_y: 3,
            bar_width: 1.4,
            meeting_rail_width: 3.0,
            split_ratio: 0.50,
            arch_spokes: 3,
            diamond_pitch: 14.0,
            acetate_thickness: 0.40,
            acetate_clearance: 0.25,
            wall_fit_clearance: 0.25,
            index_clearance: 0.15,
            plate_spacing: 10.0,
        }
    }
}

/// Derived physical measurements for assembly and acetate cutting.
#[derive(Debug, Clone)]
pub struct WindowSpecs {
    pub sleeve_outer_width: f64,
    pub sleeve_outer_height: f64,
    pub barrel_wall_thickness: f64,
    pub acetate_cut_width: f64,
    pub acetate_cut_height: f64,
    pub acetate_is_rectangular: bool,
    pub ext_seat_z: f64,
    pub ext_total_height: f64,
    pub int_total_height: f64,
    pub index_engagement: f64,
    pub effective_wall_thickness: f64,
    pub effective_ext_chamfer: f64,
    pub effective_int_chamfer: f64,
}

impl WindowParams {
    /// Applies a dollhouse scale preset relative to 1:12 default sizes.
    pub fn apply_scale_preset(&mut self, preset: ScalePreset) {
        let s = preset.scale_factor();
        let style = self.style;
        let mut base = WindowParams::default();
        style.apply_style_defaults(&mut base);

        self.opening_width = (base.opening_width * s).max(16.0);
        self.opening_height = (base.opening_height * s).max(18.0);
        self.wall_thickness = (base.wall_thickness * s).clamp(3.2, 24.0);
        self.trim_width = (base.trim_width * s).clamp(2.2, 16.0);
        self.trim_thickness = (base.trim_thickness * s).clamp(1.2, 6.0);
        self.interior_trim_width = (base.interior_trim_width * s).clamp(2.0, 16.0);
        self.interior_trim_thickness = (base.interior_trim_thickness * s).clamp(1.0, 6.0);
        self.chamfer_size = (base.chamfer_size * s).clamp(0.4, 2.5);
        self.bar_chamfer_size = (base.bar_chamfer_size * s).clamp(0.15, 0.9);
        self.groove_width = (base.groove_width * s).clamp(0.45, 1.8);
        self.door_thickness = (base.door_thickness * s).clamp(2.4, 7.0);
        self.sill_overhang = (base.sill_overhang * s).clamp(1.5, 10.0);
        self.sill_depth = (base.sill_depth * s).clamp(1.5, 10.0);
        self.frame_width = (base.frame_width * s).clamp(3.4, 14.0);
        self.bar_width = (base.bar_width * s).clamp(0.9, 4.0);
        self.meeting_rail_width = (base.meeting_rail_width * s).clamp(1.4, 6.0);
        self.diamond_pitch = (base.diamond_pitch * s).clamp(6.0, 35.0);
    }

    /// Applies style, trim, glazing, add-ons, and door hardware from `template`
    /// while preserving this opening's fitted width, height, wall/acetate thickness, and flush edges.
    pub fn apply_style_only_from(&mut self, template: &WindowParams) {
        let keep_w = self.opening_width;
        let keep_h = self.opening_height;
        let keep_wt = self.wall_thickness;
        let keep_at = self.acetate_thickness;
        let keep_ft = self.flush_top_mode;
        let keep_fb = self.flush_bottom_sill;
        let keep_fl = self.flush_left_jamb;
        let keep_fr = self.flush_right_jamb;
        let keep_fs = self.flush_trim_setback;

        *self = template.clone();
        self.opening_width = keep_w;
        self.opening_height = keep_h;
        self.wall_thickness = keep_wt;
        self.acetate_thickness = keep_at;
        self.flush_top_mode = keep_ft;
        self.flush_bottom_sill = keep_fb;
        self.flush_left_jamb = keep_fl;
        self.flush_right_jamb = keep_fr;
        self.flush_trim_setback = keep_fs;
    }

    pub fn has_any_flush_edge(&self) -> bool {
        self.flush_top_mode != FlushTopMode::None
            || self.flush_bottom_sill
            || self.flush_left_jamb
            || self.flush_right_jamb
    }

    pub fn effective_opening_mode(&self) -> WindowOpeningMode {
        if self.style.supports_opening_sash() {
            self.window_opening_mode
        } else {
            WindowOpeningMode::Fixed
        }
    }

    /// Computes sanitized parameters so geometry is always valid.
    pub fn sanitized(&self) -> Self {
        let mut p = self.clone();
        p.opening_width = p.opening_width.clamp(14.0, 260.0);
        p.opening_height = p.opening_height.clamp(14.0, 260.0);
        p.wall_thickness = p.wall_thickness.clamp(3.0, 35.0);
        p.trim_width = p.trim_width.clamp(1.5, 25.0);
        p.trim_thickness = p.trim_thickness.clamp(0.8, 10.0);
        p.interior_trim_width = p.interior_trim_width.clamp(1.5, 25.0);
        p.interior_trim_thickness = p.interior_trim_thickness.clamp(0.8, 10.0);
        p.chamfer_size = p.chamfer_size.clamp(0.2, 4.0);
        p.bar_chamfer_size = p.bar_chamfer_size.clamp(0.1, 1.2);
        p.groove_width = p.groove_width.clamp(0.4, 2.0);
        p.shutter_width_ratio = p.shutter_width_ratio.clamp(0.22, 0.55);
        p.flush_trim_setback = p.flush_trim_setback.clamp(0.0, 15.0);
        p.door_thickness = p.door_thickness.clamp(2.5, 7.5);
        p.hinge_clearance = p.hinge_clearance.clamp(0.15, 0.8);
        p.lower_panel_ratio = p.lower_panel_ratio.clamp(0.0, 0.60);
        p.letterbox_height_ratio = p.letterbox_height_ratio.clamp(0.22, 0.72);
        p.cat_flap_width = p.cat_flap_width.clamp(8.0, 26.0);
        p.cat_flap_height = p.cat_flap_height.clamp(9.0, 28.0);
        p.sill_overhang = p.sill_overhang.clamp(0.0, 15.0);
        p.sill_depth = p.sill_depth.clamp(0.5, 15.0);
        p.wall_fit_clearance = p.wall_fit_clearance.clamp(0.0, 1.0);
        p.index_clearance = p.index_clearance.clamp(0.05, 0.6);
        p.acetate_thickness = p.acetate_thickness.clamp(0.1, 5.0);
        p.acetate_clearance = p.acetate_clearance.clamp(0.05, 1.0);

        let min_dim = p.opening_width.min(p.opening_height);
        let max_frame = (min_dim * 0.34).max(3.2);
        p.frame_width = p.frame_width.clamp(3.2, max_frame);
        p.bar_width = p.bar_width.clamp(0.8, 5.0);
        p.meeting_rail_width = p.meeting_rail_width.clamp(p.bar_width, 8.0);
        p.split_ratio = p.split_ratio.clamp(0.20, 0.80);
        p.diamond_pitch = p.diamond_pitch.clamp(5.0, 40.0);
        p.plate_spacing = p.plate_spacing.clamp(2.0, 30.0);

        if p.style == WindowStyle::CircularBullsEye {
            let d = p.opening_width.min(p.opening_height);
            if (p.opening_width - p.opening_height).abs() < 0.01 {
                p.opening_width = d;
                p.opening_height = d;
            }
        }
        p
    }

    pub fn compute_specs(&self) -> WindowSpecs {
        let p = self.sanitized();
        let sleeve_w = p.opening_width - 2.0 * p.wall_fit_clearance;
        let sleeve_h = p.opening_height - 2.0 * p.wall_fit_clearance;

        let barrel_wall = (p.frame_width * 0.36).clamp(1.2, 2.2);

        let ac_inset = barrel_wall + p.acetate_clearance;
        let ac_w = (sleeve_w - 2.0 * ac_inset).max(4.0);
        let ac_h = (sleeve_h - 2.0 * ac_inset).max(4.0);

        let min_inner = if p.style.is_door() {
            (p.door_thickness + 2.4).max(p.acetate_thickness + 2.2)
        } else {
            p.acetate_thickness + 2.2
        };
        let effective_wall_thickness = p.wall_thickness.max(min_inner);
        let inner_space = effective_wall_thickness - p.acetate_thickness;
        let ext_wall_depth = (inner_space * 0.45).max(1.0);
        let int_wall_depth = (inner_space - ext_wall_depth).max(1.0);

        let barrel_reach =
            (effective_wall_thickness - 0.4).max(ext_wall_depth + p.acetate_thickness + 0.8);
        let ext_seat_z = p.trim_thickness + ext_wall_depth;
        let ext_total_height = p.trim_thickness + barrel_reach;
        let int_total_height = p.interior_trim_thickness + int_wall_depth;
        let index_engagement = (barrel_reach - ext_wall_depth - p.acetate_thickness).max(0.4);

        let effective_ext_chamfer = if p.architrave_style.has_chamfer() {
            let max_sill_c = if p.sill_enabled && p.style.supports_sill() {
                p.sill_depth * 0.6
            } else {
                p.trim_width
            };
            let max_c = (p.trim_width * 0.52)
                .min(p.trim_thickness * 0.68 / 1.15)
                .min(max_sill_c);
            let c = p.chamfer_size.min(max_c);
            if c >= 0.15 { c } else { 0.0 }
        } else {
            0.0
        };

        let effective_int_chamfer = if p.architrave_style.has_chamfer() && p.chamfer_interior {
            let max_c = (p.interior_trim_width * 0.52).min(p.interior_trim_thickness * 0.68 / 1.15);
            let c = p.chamfer_size.min(max_c);
            if c >= 0.15 { c } else { 0.0 }
        } else {
            0.0
        };

        let acetate_is_rectangular = !matches!(
            p.style,
            WindowStyle::ArchedGeorgian
                | WindowStyle::GothicPointed
                | WindowStyle::CircularBullsEye
                | WindowStyle::HalfRoundLunette
        );

        WindowSpecs {
            sleeve_outer_width: sleeve_w,
            sleeve_outer_height: sleeve_h,
            barrel_wall_thickness: barrel_wall,
            acetate_cut_width: ac_w,
            acetate_cut_height: ac_h,
            acetate_is_rectangular,
            ext_seat_z,
            ext_total_height,
            int_total_height,
            index_engagement,
            effective_wall_thickness,
            effective_ext_chamfer,
            effective_int_chamfer,
        }
    }
}
