use serde::Deserialize;

use crate::grid::Pos;
use crate::pipeline::{Fail, Stage};
use crate::rng::Rng;
use crate::world::{Marker, World};

/// Places stairs about as far apart as the map allows, with a marker for each.
/// Rejects maps where they're under `min_distance` apart or not all connected.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceStairs {
    #[serde(default)]
    pub min_distance: u32,
    #[serde(default = "default_up")]
    pub up: String,
    #[serde(default = "default_down")]
    pub down: String,
}

fn default_up() -> String {
    "stairs_up".into()
}

fn default_down() -> String {
    "stairs_down".into()
}

impl Stage for PlaceStairs {
    fn run(&self, world: &mut World, rng: &mut Rng) -> Result<(), Fail> {
        let (up_tag, down_tag) = (world.tag(&self.up)?, world.tag(&self.down)?);
        let walkable: Vec<Pos> = world
            .terrain
            .positions()
            .filter(|&p| world.walkable(p))
            .collect();
        if walkable.is_empty() {
            return Err(Fail::reject("no walkable tiles"));
        }
        let probe = walkable[rng.index(walkable.len())];
        let (up, _, _) = farthest(world, probe);
        let (down, distance, reached) = farthest(world, up);
        if reached < walkable.len() {
            return Err(Fail::reject("map is not fully connected"));
        }
        if distance < self.min_distance {
            return Err(Fail::reject(format!(
                "stairs only {distance} apart (need {})",
                self.min_distance
            )));
        }
        world.terrain.set(up, up_tag);
        world.terrain.set(down, down_tag);
        world.markers.push(Marker {
            kind: self.up.clone(),
            pos: up,
        });
        world.markers.push(Marker {
            kind: self.down.clone(),
            pos: down,
        });
        Ok(())
    }
}

/// Farthest reachable tile, its distance, and how many tiles were reachable.
fn farthest(world: &World, from: Pos) -> (Pos, u32, usize) {
    let mut best = (from, 0);
    let mut reached = 0;
    for (i, d) in world.distances_from(from).into_iter().enumerate() {
        if let Some(d) = d {
            reached += 1;
            if d > best.1 {
                best = (world.terrain.pos(i), d);
            }
        }
    }
    (best.0, best.1, reached)
}

/// Places `count` tiles of `tag` on tiles currently tagged with one of `on`,
/// at least `min_spacing` apart (Manhattan distance), each with a marker named
/// after the tag. Rejects the map if they don't all fit.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Place {
    pub tag: String,
    pub on: Vec<String>,
    pub count: u32,
    #[serde(default)]
    pub min_spacing: i32,
}

impl Stage for Place {
    fn run(&self, world: &mut World, rng: &mut Rng) -> Result<(), Fail> {
        let tag = world.tag(&self.tag)?;
        let on = self
            .on
            .iter()
            .map(|n| world.tag(n))
            .collect::<Result<Vec<_>, _>>()?;
        let candidates: Vec<Pos> = world
            .terrain
            .positions()
            .filter(|&p| world.terrain.get(p).is_some_and(|t| on.contains(&t)))
            .collect();
        let count = self.count as usize;
        let mut placed: Vec<Pos> = Vec::with_capacity(count);
        if !candidates.is_empty() {
            for _ in 0..count * 50 {
                if placed.len() == count {
                    break;
                }
                let p = candidates[rng.index(candidates.len())];
                if placed
                    .iter()
                    .all(|&q| q != p && p.manhattan(q) >= self.min_spacing)
                {
                    placed.push(p);
                }
            }
        }
        if placed.len() < count {
            return Err(Fail::reject(format!(
                "only fit {} of {count} `{}`",
                placed.len(),
                self.tag
            )));
        }
        for pos in placed {
            world.terrain.set(pos, tag);
            world.markers.push(Marker {
                kind: self.tag.clone(),
                pos,
            });
        }
        Ok(())
    }
}
