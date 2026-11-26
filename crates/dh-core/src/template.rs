use crate::params::WindowParams;
use crate::project::{new_id, now_epoch};
use serde::{Deserialize, Serialize};

/// Scope where a saved template is stored and discoverable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateScope {
    Personal,
    Project,
}

impl TemplateScope {
    pub fn label(self) -> &'static str {
        match self {
            TemplateScope::Personal => "Personal Library",
            TemplateScope::Project => "Project Templates",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateCategory {
    Window,
    Door,
}

/// A reusable pre-parameterised window or hinged door preset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PieceTemplate {
    pub id: String,
    pub name: String,
    pub category: TemplateCategory,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub notes: String,
    pub params: WindowParams,
    #[serde(default)]
    pub created_at_epoch: u64,
    #[serde(default)]
    pub updated_at_epoch: u64,
}

impl PieceTemplate {
    pub fn from_params(
        name: impl Into<String>,
        notes: impl Into<String>,
        tags: Vec<String>,
        params: WindowParams,
    ) -> Self {
        let now = now_epoch();
        let category = if params.style.is_door() {
            TemplateCategory::Door
        } else {
            TemplateCategory::Window
        };
        Self {
            id: new_id("tpl"),
            name: name.into(),
            category,
            tags,
            notes: notes.into(),
            params,
            created_at_epoch: now,
            updated_at_epoch: now,
        }
    }
}
