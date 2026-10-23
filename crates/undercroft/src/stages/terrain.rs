use serde::Deserialize;

use crate::grid::Pos;
use crate::pipeline::{Fail, Stage};
use crate::rng::Rng;
use crate::world::World;

/// Sets every tile to `tag`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fill {
    pub tag: String,
}

impl Stage for Fill {
    fn run(&self, world: &mut World, _rng: &mut Rng) -> Result<(), Fail> {
        let tag = world.tag(&self.tag)?;
        world.terrain.cells_mut().fill(tag);
        Ok(())
    }
}

/// Sets each tile except the outer border to `tag` with probability `chance`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RandomFill {
    pub tag: String,
    pub chance: f64,
}

impl Stage for RandomFill {
    fn run(&self, world: &mut World, rng: &mut Rng) -> Result<(), Fail> {
        let tag = world.tag(&self.tag)?;
        for y in 1..world.height() - 1 {
            for x in 1..world.width() - 1 {
                if rng.chance(self.chance) {
                    world.terrain.set(Pos { x, y }, tag);
                }
            }
        }
        Ok(())
    }
}

/// Cellular-automata smoothing with the "4-5 rule": a `solid` tile stays
/// solid with at least 4 solid neighbours, any other tile turns solid with at
/// least 5, and everything else becomes `open`. Off-map counts as solid.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CellularSmooth {
    pub steps: u32,
    pub solid: String,
    pub open: String,
}

impl Stage for CellularSmooth {
    fn run(&self, world: &mut World, _rng: &mut Rng) -> Result<(), Fail> {
        let (solid, open) = (world.tag(&self.solid)?, world.tag(&self.open)?);
        for _ in 0..self.steps {
            let prev = world.terrain.clone();
            for p in prev.positions() {
                let mut n = 0;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let q = Pos {
                            x: p.x + dx,
                            y: p.y + dy,
                        };
                        if (dx, dy) != (0, 0) && prev.get(q).is_none_or(|t| t == solid) {
                            n += 1;
                        }
                    }
                }
                let was_solid = prev.get(p) == Some(solid);
                let solid_now = if was_solid { n >= 4 } else { n >= 5 };
                world.terrain.set(p, if solid_now { solid } else { open });
            }
        }
        Ok(())
    }
}

/// Keeps the largest connected walkable region and turns every other
/// walkable tile into `fill`. Ties go to the region found first.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeepLargestRegion {
    pub fill: String,
}

impl Stage for KeepLargestRegion {
    fn run(&self, world: &mut World, _rng: &mut Rng) -> Result<(), Fail> {
        let fill = world.tag(&self.fill)?;
        let regions = world.regions();
        let mut best = 0;
        for (i, region) in regions.iter().enumerate() {
            if region.len() > regions[best].len() {
                best = i;
            }
        }
        let cells = world.terrain.cells_mut();
        for (i, region) in regions.iter().enumerate() {
            if i != best {
                for &j in region {
                    cells[j] = fill;
                }
            }
        }
        Ok(())
    }
}
