use std::ops::Range;

use serde::Deserialize;

use crate::grid::{Pos, Rect};
use crate::pipeline::{Fail, Stage};
use crate::rng::Rng;
use crate::tags::Tag;
use crate::world::{Area, World};

/// Binary space partitioning: one room per leaf, a corridor across every split
/// so all rooms connect. `loops` is the chance of an extra corridor between
/// nearby rooms. Rooms are recorded as `room` areas.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BspRooms {
    pub min_leaf: i32,
    pub min_room: i32,
    pub max_room: i32,
    #[serde(default)]
    pub loops: f64,
    #[serde(default = "default_floor")]
    pub floor: String,
}

fn default_floor() -> String {
    "floor".into()
}

impl Stage for BspRooms {
    fn run(&self, world: &mut World, rng: &mut Rng) -> Result<(), Fail> {
        if self.min_room < 1 || self.max_room < self.min_room || self.min_leaf < self.min_room + 2 {
            return Err(Fail::invalid(
                "needs 1 <= min_room <= max_room and min_leaf >= min_room + 2",
            ));
        }
        if world.width() < self.min_leaf || world.height() < self.min_leaf {
            return Err(Fail::invalid("map is smaller than min_leaf"));
        }
        let floor = world.tag(&self.floor)?;
        let root = Rect {
            x: 0,
            y: 0,
            w: world.width(),
            h: world.height(),
        };
        let mut rooms = Vec::new();
        self.split(root, world, rng, floor, &mut rooms);
        // Rooms two apart in leaf order are close, so these loops stay short.
        for i in 0..rooms.len().saturating_sub(2) {
            if rng.chance(self.loops) {
                corridor(world, rng, floor, rooms[i].center(), rooms[i + 2].center());
            }
        }
        world.areas.extend(rooms.into_iter().map(|rect| Area {
            kind: "room".into(),
            rect,
        }));
        Ok(())
    }
}

impl BspRooms {
    /// Returns the indices into `rooms` created inside `rect`.
    fn split(
        &self,
        rect: Rect,
        world: &mut World,
        rng: &mut Rng,
        floor: Tag,
        rooms: &mut Vec<Rect>,
    ) -> Range<usize> {
        let start = rooms.len();
        let can_x = rect.w >= 2 * self.min_leaf;
        let can_y = rect.h >= 2 * self.min_leaf;
        let split_x = match (can_x, can_y) {
            (false, false) => {
                rooms.push(self.carve_room(rect, world, rng, floor));
                return start..rooms.len();
            }
            (true, false) => true,
            (false, true) => false,
            (true, true) if rect.w == rect.h => rng.chance(0.5),
            (true, true) => rect.w > rect.h,
        };
        let (a, b) = if split_x {
            let at = rng.range(self.min_leaf, rect.w - self.min_leaf + 1);
            (
                Rect { w: at, ..rect },
                Rect {
                    x: rect.x + at,
                    w: rect.w - at,
                    ..rect
                },
            )
        } else {
            let at = rng.range(self.min_leaf, rect.h - self.min_leaf + 1);
            (
                Rect { h: at, ..rect },
                Rect {
                    y: rect.y + at,
                    h: rect.h - at,
                    ..rect
                },
            )
        };
        let left = self.split(a, world, rng, floor, rooms);
        let right = self.split(b, world, rng, floor, rooms);
        let from = rooms[left.start + rng.index(left.len())].center();
        let to = rooms[right.start + rng.index(right.len())].center();
        corridor(world, rng, floor, from, to);
        start..rooms.len()
    }

    /// Carves a room inside `leaf`, leaving at least one tile of wall around it.
    fn carve_room(&self, leaf: Rect, world: &mut World, rng: &mut Rng, floor: Tag) -> Rect {
        let w = rng.range(self.min_room, self.max_room.min(leaf.w - 2) + 1);
        let h = rng.range(self.min_room, self.max_room.min(leaf.h - 2) + 1);
        let x = leaf.x + 1 + rng.range(0, leaf.w - 2 - w + 1);
        let y = leaf.y + 1 + rng.range(0, leaf.h - 2 - h + 1);
        for ry in y..y + h {
            for rx in x..x + w {
                world.terrain.set(Pos { x: rx, y: ry }, floor);
            }
        }
        Rect { x, y, w, h }
    }
}

/// An L-shaped corridor, bending at a random one of the two possible corners.
fn corridor(world: &mut World, rng: &mut Rng, tag: Tag, a: Pos, b: Pos) {
    let corner = if rng.chance(0.5) {
        Pos { x: b.x, y: a.y }
    } else {
        Pos { x: a.x, y: b.y }
    };
    line(world, tag, a, corner);
    line(world, tag, corner, b);
}

fn line(world: &mut World, tag: Tag, a: Pos, b: Pos) {
    let (dx, dy) = ((b.x - a.x).signum(), (b.y - a.y).signum());
    let mut p = a;
    world.terrain.set(p, tag);
    while p != b {
        p.x += dx;
        p.y += dy;
        world.terrain.set(p, tag);
    }
}
