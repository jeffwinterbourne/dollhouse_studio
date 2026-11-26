use serde::{Deserialize, Serialize};

const MM_PER_CM: f64 = 10.0;
const MM_PER_INCH: f64 = 25.4;

/// User-facing length unit for display, dimensions, rulers, and photo calibration.
/// All internal geometry and CAD coordinates are stored strictly in exact millimeters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum DisplayUnit {
    #[default]
    Mm,
    Cm,
    Inches,
}

impl DisplayUnit {
    pub const ALL: [DisplayUnit; 3] = [DisplayUnit::Mm, DisplayUnit::Cm, DisplayUnit::Inches];

    pub fn short_label(self) -> &'static str {
        match self {
            DisplayUnit::Mm => "mm",
            DisplayUnit::Cm => "cm",
            DisplayUnit::Inches => "in",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DisplayUnit::Mm => "Millimeters (mm)",
            DisplayUnit::Cm => "Centimeters (cm)",
            DisplayUnit::Inches => "Inches (in)",
        }
    }

    pub fn suffix(self) -> &'static str {
        match self {
            DisplayUnit::Mm => " mm",
            DisplayUnit::Cm => " cm",
            DisplayUnit::Inches => " in",
        }
    }

    pub fn decimals(self) -> usize {
        match self {
            DisplayUnit::Mm => 1,
            DisplayUnit::Cm => 2,
            DisplayUnit::Inches => 3,
        }
    }

    pub fn drag_speed(self) -> f64 {
        match self {
            DisplayUnit::Mm => 0.1,
            DisplayUnit::Cm => 0.02,
            DisplayUnit::Inches => 0.01,
        }
    }

    pub fn from_mm(self, mm: f64) -> f64 {
        match self {
            DisplayUnit::Mm => mm,
            DisplayUnit::Cm => mm / MM_PER_CM,
            DisplayUnit::Inches => mm / MM_PER_INCH,
        }
    }

    pub fn to_mm(self, value: f64) -> f64 {
        match self {
            DisplayUnit::Mm => value,
            DisplayUnit::Cm => value * MM_PER_CM,
            DisplayUnit::Inches => value * MM_PER_INCH,
        }
    }

    pub fn format_value(self, mm: f64) -> String {
        let v = self.from_mm(mm);
        match self {
            DisplayUnit::Mm => format!("{v:.1} mm"),
            DisplayUnit::Cm => format!("{v:.2} cm"),
            DisplayUnit::Inches => format!("{v:.3} in"),
        }
    }

    pub fn format_dims(self, w_mm: f64, h_mm: f64) -> String {
        let w = self.from_mm(w_mm);
        let h = self.from_mm(h_mm);
        match self {
            DisplayUnit::Mm => format!("{w:.1} × {h:.1} mm"),
            DisplayUnit::Cm => format!("{w:.2} × {h:.2} cm"),
            DisplayUnit::Inches => format!("{w:.2} × {h:.2} in"),
        }
    }

    /// Major grid spacing in millimeters that corresponds to clean unit intervals.
    pub fn ruler_major_step_mm(self) -> f64 {
        match self {
            DisplayUnit::Mm => 50.0,
            DisplayUnit::Cm => 50.0,
            DisplayUnit::Inches => MM_PER_INCH,
        }
    }

    /// Minor grid spacing in millimeters.
    pub fn ruler_minor_step_mm(self) -> f64 {
        match self {
            DisplayUnit::Mm => 10.0,
            DisplayUnit::Cm => 10.0,
            DisplayUnit::Inches => MM_PER_INCH * 0.25,
        }
    }

    /// One-click calibration preset chips `(label, real_length_mm)`.
    pub fn calibration_presets(self) -> Vec<(&'static str, f64)> {
        match self {
            DisplayUnit::Mm => vec![
                ("50 mm", 50.0),
                ("100 mm", 100.0),
                ("150 mm", 150.0),
                ("200 mm", 200.0),
                ("Card W", 85.6),
                ("1:12 Door H", 166.0),
            ],
            DisplayUnit::Cm => vec![
                ("5 cm", 50.0),
                ("10 cm", 100.0),
                ("15 cm", 150.0),
                ("20 cm", 200.0),
                ("Card W", 85.6),
                ("1:12 Door H", 166.0),
            ],
            DisplayUnit::Inches => vec![
                ("1 in", 25.4),
                ("2 in", 50.8),
                ("6 in", 152.4),
                ("12 in", 304.8),
                ("Card W", 85.6),
                ("1:12 Door H", 166.0),
            ],
        }
    }
}
