use std::collections::{BTreeMap, VecDeque};
use std::fmt;

use serde::Serialize;

use crate::grid::{Grid, Pos, Rect};
use crate::pipeline::Fail;
use crate::tags::{Tag, TagSet};

/// Everything a recipe produces. Stages read and write it in order.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct World {
    /// Serialized as a string: u64 doesn't fit in a JavaScript number.
    #[serde(serialize_with = "serialize_u64_as_string")]
    pub seed: u64,
    /// How many rejected attempts came before this one.
    pub attempt: u32,
    pub tags: TagSet,
    /// The main tile layer, as tag ids into `tags`.
    pub terrain: Grid<Tag>,
    /// Named scalar layers such as `height` or `moisture`.
    pub fields: BTreeMap<String, Grid<f32>>,
    /// Named rectangles such as rooms.
    pub areas: Vec<Area>,
    /// Named points such as stairs or towns.
    pub markers: Vec<Marker>,
}

fn serialize_u64_as_string<S: serde::Serializer>(v: &u64, s: S) -> Result<S::Ok, S::Error> {
    s.collect_str(v)
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Area {
    pub kind: String,
    pub rect: Rect,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Marker {
    pub kind: String,
    pub pos: Pos,
}

impl World {
    /// An empty world, filled with `void`.
    pub fn new(seed: u64, attempt: u32, width: i32, height: i32, tags: TagSet) -> Self {
        World {
            seed,
            attempt,
            tags,
            terrain: Grid::new(width, height, Tag(0)),
            fields: BTreeMap::new(),
            areas: Vec::new(),
            markers: Vec::new(),
        }
    }

    pub fn width(&self) -> i32 {
        self.terrain.width()
    }

    pub fn height(&self) -> i32 {
        self.terrain.height()
    }

    /// Looks up a tag by name. An unknown name is a recipe error, not a bad roll.
    pub fn tag(&self, name: &str) -> Result<Tag, Fail> {
        self.tags
            .get(name)
            .ok_or_else(|| Fail::Invalid(format!("unknown tag `{name}`")))
    }

    pub fn field(&self, name: &str) -> Result<&Grid<f32>, Fail> {
        self.fields.get(name).ok_or_else(|| {
            Fail::Invalid(format!(
                "no field named `{name}` (add a stage that writes it first)"
            ))
        })
    }

    pub fn walkable(&self, p: Pos) -> bool {
        self.terrain
            .get(p)
            .is_some_and(|t| self.tags.def(t).walkable)
    }

    pub fn marker(&self, kind: &str) -> Option<Pos> {
        self.markers.iter().find(|m| m.kind == kind).map(|m| m.pos)
    }

    /// Walking distance to every cell, indexed like the terrain grid.
    pub fn distances_from(&self, start: Pos) -> Vec<Option<u32>> {
        let mut dist = vec![None; self.terrain.cells().len()];
        let Some(i) = self.terrain.index(start) else {
            return dist;
        };
        if !self.walkable(start) {
            return dist;
        }
        dist[i] = Some(0);
        let mut queue = VecDeque::from([(start, 0)]);
        while let Some((p, d)) = queue.pop_front() {
            for n in p.neighbors4() {
                if let Some(j) = self.terrain.index(n) {
                    if dist[j].is_none() && self.walkable(n) {
                        dist[j] = Some(d + 1);
                        queue.push_back((n, d + 1));
                    }
                }
            }
        }
        dist
    }

    /// Connected walkable regions as lists of cell indices.
    pub fn regions(&self) -> Vec<Vec<usize>> {
        let mut seen = vec![false; self.terrain.cells().len()];
        let mut regions = Vec::new();
        for p in self.terrain.positions() {
            let i = self.terrain.index(p).unwrap();
            if seen[i] || !self.walkable(p) {
                continue;
            }
            let region: Vec<usize> = self
                .distances_from(p)
                .iter()
                .enumerate()
                .filter_map(|(j, d)| d.map(|_| j))
                .collect();
            for &j in &region {
                seen[j] = true;
            }
            regions.push(region);
        }
        regions
    }
}

impl fmt::Display for World {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in self.terrain.cells().chunks(self.width() as usize) {
            let line: String = row.iter().map(|&t| self.tags.def(t).glyph).collect();
            writeln!(f, "{}", line.trim_end())?;
        }
        Ok(())
    }
}
