use serde::Deserialize;

use crate::pipeline::{Fail, Stage};
use crate::rng::Rng;
use crate::world::World;

/// Rejects the map unless every walkable tile can reach every other.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Connected {}

impl Stage for Connected {
    fn run(&self, world: &mut World, _rng: &mut Rng) -> Result<(), Fail> {
        match world.regions().len() {
            0 => Err(Fail::reject("no walkable tiles")),
            1 => Ok(()),
            n => Err(Fail::reject(format!("{n} disconnected regions"))),
        }
    }
}

/// Rejects the map unless the share of tiles tagged with one of `tags` is
/// between `min` and `max` (inclusive).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    pub tags: Vec<String>,
    #[serde(default)]
    pub min: f64,
    #[serde(default = "default_max")]
    pub max: f64,
}

fn default_max() -> f64 {
    1.0
}

impl Stage for Coverage {
    fn run(&self, world: &mut World, _rng: &mut Rng) -> Result<(), Fail> {
        let tags = self
            .tags
            .iter()
            .map(|n| world.tag(n))
            .collect::<Result<Vec<_>, _>>()?;
        let cells = world.terrain.cells();
        let hits = cells.iter().filter(|t| tags.contains(t)).count();
        let share = hits as f64 / cells.len() as f64;
        if share < self.min || share > self.max {
            return Err(Fail::reject(format!(
                "{:.0}% coverage, want {:.0}%..={:.0}%",
                share * 100.0,
                self.min * 100.0,
                self.max * 100.0
            )));
        }
        Ok(())
    }
}
