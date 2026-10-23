//! Deterministic roguelike rules on top of undercroft dungeons.
//!
//! A run is fully described by its seed and the list of actions taken, so
//! [`Game::replay`] reproduces any run exactly.

pub mod bot;
mod content;
mod fov;
mod game;

pub use content::{ItemKind, MONSTERS, MonsterKind};
pub use game::{Action, FLOORS, Game, SpriteView, Status, View, decode, encode};
