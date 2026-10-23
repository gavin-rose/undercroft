//! Seeded, validated procedural generation built from composable stages.
//!
//! A [`Recipe`] is a list of [`Stage`]s that each read and write a [`World`]:
//! a tile map of semantic tags plus named scalar fields, areas and markers.
//! Styles such as `dungeon` and `overworld` are just built-in recipes, so the
//! same engine (and the same renderers) work for any kind of map.
//!
//! Generation is deterministic: the same seed, recipe and [`GEN_VERSION`]
//! always produce the same world, on every platform.
//!
//! ```
//! use undercroft::{Recipe, Seed, generate};
//!
//! let recipe = Recipe::style("dungeon").unwrap();
//! let world = generate(Seed::from_text("hello"), &recipe).unwrap();
//! println!("{world}");
//! ```

mod error;
pub mod grid;
pub mod noise;
mod pipeline;
mod recipe;
mod rng;
mod seed;
pub mod stages;
mod tags;
mod world;

pub use error::Error;
pub use grid::{Grid, Pos, Rect};
pub use pipeline::{Fail, MAX_ATTEMPTS, Registry, Stage, generate, generate_with};
pub use recipe::{GEN_VERSION, Recipe, STYLES, StageSpec};
pub use rng::Rng;
pub use seed::Seed;
pub use tags::{Tag, TagDef, TagSet};
pub use world::{Area, Marker, World};
