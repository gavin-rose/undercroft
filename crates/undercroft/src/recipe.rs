use std::collections::BTreeMap;

use serde::Deserialize;

use crate::error::Error;
use crate::tags::{TagDef, TagSet};

/// Bump this whenever a change would alter the output for an existing seed
/// and recipe. Recipes declare the version they target.
pub const GEN_VERSION: u32 = 1;

/// Built-in styles: `(name, recipe source)`.
pub const STYLES: &[(&str, &str)] = &[
    ("dungeon", include_str!("../styles/dungeon.toml")),
    ("caves", include_str!("../styles/caves.toml")),
    ("overworld", include_str!("../styles/overworld.toml")),
];

/// A list of stages to run, plus the map size and any custom tags.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    pub gen_version: u32,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub width: i32,
    pub height: i32,
    #[serde(default)]
    pub tags: BTreeMap<String, TagDef>,
    #[serde(rename = "stage")]
    pub stages: Vec<StageSpec>,
}

/// One `[[stage]]` table: `use` names the stage, everything else is its parameters.
#[derive(Clone, Debug, Deserialize)]
pub struct StageSpec {
    #[serde(rename = "use")]
    pub kind: String,
    /// Optional stable name for seeding. Set it when a recipe uses the same
    /// stage twice and you want to reorder them without changing output.
    #[serde(default)]
    pub id: Option<String>,
    #[serde(flatten)]
    pub params: toml::Table,
}

impl Recipe {
    pub fn from_toml(source: &str) -> Result<Self, Error> {
        Ok(toml::from_str(source)?)
    }

    /// A built-in style from [`STYLES`].
    pub fn style(name: &str) -> Option<Self> {
        let (_, source) = STYLES.iter().find(|(n, _)| *n == name)?;
        Some(Recipe::from_toml(source).expect("built-in styles are valid"))
    }

    pub fn tag_set(&self) -> TagSet {
        let mut tags = TagSet::builtin();
        for (name, def) in &self.tags {
            tags.define(name, def.clone());
        }
        tags
    }

    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.gen_version != GEN_VERSION {
            return Err(Error::UnsupportedVersion(self.gen_version));
        }
        let ok = 8..=2048;
        if !ok.contains(&self.width) || !ok.contains(&self.height) {
            return Err(Error::InvalidRecipe(format!(
                "size {}x{} is outside 8..=2048",
                self.width, self.height
            )));
        }
        if self.stages.is_empty() {
            return Err(Error::InvalidRecipe("recipe has no stages".into()));
        }
        Ok(())
    }
}
