use crate::params::{FlushTopMode, WindowParams};
use crate::project::{new_id, now_epoch};
use serde::{Deserialize, Serialize};

/// Shared live CAD Component definition whose edits propagate to every linked instance
/// across one or multiple walls.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub notes: String,
    pub params: WindowParams,
    #[serde(default)]
    pub updated_at_epoch: u64,
}

impl ComponentDef {
    pub fn new(name: impl Into<String>, params: WindowParams) -> Self {
        Self {
            id: new_id("comp"),
            name: name.into(),
            notes: String::new(),
            params,
            updated_at_epoch: now_epoch(),
        }
    }
}

/// Optional per-instance exceptions layered on top of a shared `ComponentDef`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct InstanceOverrides {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wall_thickness: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acetate_thickness: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hinge_right: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flush_top_mode: Option<FlushTopMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flush_bottom_sill: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flush_left_jamb: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flush_right_jamb: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flush_trim_setback: Option<f64>,
}

impl InstanceOverrides {
    pub fn is_empty(&self) -> bool {
        self.wall_thickness.is_none()
            && self.acetate_thickness.is_none()
            && self.hinge_right.is_none()
            && self.flush_top_mode.is_none()
            && self.flush_bottom_sill.is_none()
            && self.flush_left_jamb.is_none()
            && self.flush_right_jamb.is_none()
            && self.flush_trim_setback.is_none()
    }

    pub fn has_flush_override(&self) -> bool {
        self.flush_top_mode.is_some()
            || self.flush_bottom_sill.is_some()
            || self.flush_left_jamb.is_some()
            || self.flush_right_jamb.is_some()
            || self.flush_trim_setback.is_some()
    }

    pub fn clear_flush_overrides(&mut self) {
        self.flush_top_mode = None;
        self.flush_bottom_sill = None;
        self.flush_left_jamb = None;
        self.flush_right_jamb = None;
        self.flush_trim_setback = None;
    }

    /// Applies any overridden fields onto `params`.
    pub fn apply_to(&self, params: &mut WindowParams) {
        if let Some(v) = self.wall_thickness {
            params.wall_thickness = v;
        }
        if let Some(v) = self.acetate_thickness {
            params.acetate_thickness = v;
        }
        if let Some(v) = self.hinge_right {
            params.hinge_right = v;
        }
        if let Some(v) = self.flush_top_mode {
            params.flush_top_mode = v;
        }
        if let Some(v) = self.flush_bottom_sill {
            params.flush_bottom_sill = v;
        }
        if let Some(v) = self.flush_left_jamb {
            params.flush_left_jamb = v;
        }
        if let Some(v) = self.flush_right_jamb {
            params.flush_right_jamb = v;
        }
        if let Some(v) = self.flush_trim_setback {
            params.flush_trim_setback = v;
        }
    }
}

/// Source of a `WallOpening`'s geometry parameters: either linked to a shared `ComponentDef`
/// or an independent unique piece.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OpeningSource {
    Component {
        component_id: String,
        #[serde(default)]
        overrides: InstanceOverrides,
    },
    Unique {
        params: WindowParams,
    },
}

impl OpeningSource {
    pub fn component_id(&self) -> Option<&str> {
        match self {
            OpeningSource::Component { component_id, .. } => Some(component_id.as_str()),
            OpeningSource::Unique { .. } => None,
        }
    }

    pub fn is_component(&self) -> bool {
        matches!(self, OpeningSource::Component { .. })
    }

    pub fn resolve(&self, components: &[ComponentDef]) -> WindowParams {
        match self {
            OpeningSource::Unique { params } => params.clone(),
            OpeningSource::Component {
                component_id,
                overrides,
            } => {
                let mut base = components
                    .iter()
                    .find(|c| &c.id == component_id)
                    .map(|c| c.params.clone())
                    .unwrap_or_default();
                overrides.apply_to(&mut base);
                base
            }
        }
    }
}
