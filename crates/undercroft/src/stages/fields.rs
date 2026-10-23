use std::collections::BTreeMap;

use serde::Deserialize;

use crate::grid::Grid;
use crate::noise;
use crate::pipeline::{Fail, Stage};
use crate::rng::Rng;
use crate::world::World;

/// Writes fractal value noise in `0..1` to the scalar field `field`.
/// Larger `scale` gives larger features.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoiseField {
    pub field: String,
    pub scale: f64,
    #[serde(default = "default_octaves")]
    pub octaves: u32,
}

fn default_octaves() -> u32 {
    4
}

impl Stage for NoiseField {
    fn run(&self, world: &mut World, rng: &mut Rng) -> Result<(), Fail> {
        if self.scale <= 0.0 {
            return Err(Fail::invalid("scale must be positive"));
        }
        let seed = rng.next_u64();
        let mut grid = Grid::new(world.width(), world.height(), 0.0f32);
        for p in grid.positions() {
            let (x, y) = (p.x as f64 / self.scale, p.y as f64 / self.scale);
            grid.set(p, noise::fractal(seed, x, y, self.octaves) as f32);
        }
        world.fields.insert(self.field.clone(), grid);
        Ok(())
    }
}

/// Lowers `field` towards the map edges so high ground gathers in the middle.
/// `strength` 1.0 takes the corners all the way to zero.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IslandMask {
    pub field: String,
    #[serde(default = "default_strength")]
    pub strength: f64,
}

fn default_strength() -> f64 {
    1.0
}

impl Stage for IslandMask {
    fn run(&self, world: &mut World, _rng: &mut Rng) -> Result<(), Fail> {
        let (w, h) = (world.width() as f64, world.height() as f64);
        world.field(&self.field)?;
        let grid = world.fields.get_mut(&self.field).unwrap();
        for p in grid.positions() {
            let dx = (p.x as f64 + 0.5) / w * 2.0 - 1.0;
            let dy = (p.y as f64 + 0.5) / h * 2.0 - 1.0;
            let falloff = 1.0 - self.strength * (dx * dx + dy * dy).min(1.0);
            let v = grid.get(p).unwrap() as f64 * falloff;
            grid.set(p, v.max(0.0) as f32);
        }
        Ok(())
    }
}

/// Assigns terrain from scalar fields. Rules are checked in order and the
/// first match wins; tiles that match no rule are left alone.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Classify {
    pub rules: Vec<ClassifyRule>,
}

/// A `tag` plus any number of `field = [min, max]` ranges (min inclusive,
/// max exclusive). A rule with no ranges matches everything.
#[derive(Debug, Deserialize)]
pub struct ClassifyRule {
    pub tag: String,
    #[serde(flatten)]
    pub ranges: BTreeMap<String, [f32; 2]>,
}

impl Stage for Classify {
    fn run(&self, world: &mut World, _rng: &mut Rng) -> Result<(), Fail> {
        let mut rules = Vec::with_capacity(self.rules.len());
        for rule in &self.rules {
            let tag = world.tag(&rule.tag)?;
            let mut ranges = Vec::with_capacity(rule.ranges.len());
            for (name, &range) in &rule.ranges {
                ranges.push((world.field(name)?, range));
            }
            rules.push((tag, ranges));
        }
        let mut terrain = world.terrain.clone();
        for p in terrain.positions() {
            let matched = rules.iter().find(|(_, ranges)| {
                ranges.iter().all(|(field, [lo, hi])| {
                    let v = field.get(p).unwrap();
                    v >= *lo && v < *hi
                })
            });
            if let Some((tag, _)) = matched {
                terrain.set(p, *tag);
            }
        }
        world.terrain = terrain;
        Ok(())
    }
}
