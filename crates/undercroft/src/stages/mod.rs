//! Built-in stages. Each is a plain struct deserialized from its `[[stage]]` table.

mod checks;
mod features;
mod fields;
mod rooms;
mod terrain;

pub use checks::{Connected, Coverage};
pub use features::{Place, PlaceStairs};
pub use fields::{Classify, ClassifyRule, IslandMask, NoiseField};
pub use rooms::BspRooms;
pub use terrain::{CellularSmooth, Fill, KeepLargestRegion, RandomFill};

use crate::pipeline::Registry;

pub(crate) fn register_builtins(registry: &mut Registry) {
    registry.register::<Fill>("fill");
    registry.register::<RandomFill>("random_fill");
    registry.register::<CellularSmooth>("cellular_smooth");
    registry.register::<KeepLargestRegion>("keep_largest_region");
    registry.register::<BspRooms>("bsp_rooms");
    registry.register::<NoiseField>("noise_field");
    registry.register::<IslandMask>("island_mask");
    registry.register::<Classify>("classify");
    registry.register::<PlaceStairs>("place_stairs");
    registry.register::<Place>("place");
    registry.register::<Connected>("connected");
    registry.register::<Coverage>("coverage");
}
