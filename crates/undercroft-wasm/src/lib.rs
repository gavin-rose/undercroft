//! WebAssembly bindings. Build with `web/build.sh`.

use std::cell::OnceCell;

use undercroft::{Recipe, STYLES, Seed, World};
use undercroft_render::Tileset;
use undercroft_sim::{Action, Game, encode};
use wasm_bindgen::prelude::*;

thread_local! {
    static TILESET: OnceCell<Tileset> = const { OnceCell::new() };
}

#[wasm_bindgen]
pub struct Generated {
    world: World,
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

#[wasm_bindgen]
impl Generated {
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// RGBA pixels, row-major, for an `ImageData`.
    pub fn pixels(&self) -> Vec<u8> {
        self.pixels.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn attempt(&self) -> u32 {
        self.world.attempt
    }

    #[wasm_bindgen(getter)]
    pub fn seed(&self) -> String {
        self.world.seed.to_string()
    }

    pub fn ascii(&self) -> String {
        self.world.to_string()
    }

    pub fn json(&self) -> String {
        serde_json::to_string(&self.world).expect("worlds always serialize")
    }
}

#[wasm_bindgen]
pub fn generate(recipe_toml: &str, seed_text: &str) -> Result<Generated, JsError> {
    run(recipe_toml, Seed::from_text(seed_text))
}

/// Generates the shared daily world for `date` (`YYYY-MM-DD`).
#[wasm_bindgen(js_name = generateDaily)]
pub fn generate_daily(recipe_toml: &str, date: &str) -> Result<Generated, JsError> {
    run(recipe_toml, Seed::daily(date))
}

/// The built-in styles as a JSON array of `{ name, description, source }`.
#[wasm_bindgen]
pub fn styles() -> String {
    let list: Vec<_> = STYLES
        .iter()
        .map(|&(name, source)| {
            let description = Recipe::style(name).and_then(|r| r.description);
            serde_json::json!({ "name": name, "description": description, "source": source })
        })
        .collect();
    serde_json::to_string(&list).unwrap()
}

#[wasm_bindgen(js_name = genVersion)]
pub fn gen_version() -> u32 {
    undercroft::GEN_VERSION
}

/// 256x256 tiles renders to a 4096px square, about 64 MB of RGBA.
const MAX_TILES: i64 = 256 * 256;

fn run(recipe_toml: &str, seed: Seed) -> Result<Generated, JsError> {
    let recipe = Recipe::from_toml(recipe_toml)?;
    if recipe.width as i64 * recipe.height as i64 > MAX_TILES {
        return Err(JsError::new(&format!(
            "{}x{} is too big for the browser (max {MAX_TILES} tiles); use the CLI for huge maps",
            recipe.width, recipe.height
        )));
    }
    let world = undercroft::generate(seed, &recipe)?;
    let image = TILESET.with(|t| t.get_or_init(Tileset::builtin).render(&world, 1));
    Ok(Generated {
        width: image.width(),
        height: image.height(),
        pixels: image.into_raw(),
        world,
    })
}

#[wasm_bindgen]
pub struct Play {
    game: Game,
}

#[wasm_bindgen]
impl Play {
    /// The shared daily run for `date` (`YYYY-MM-DD`).
    pub fn daily(date: &str) -> Play {
        Play {
            game: Game::new(Seed::daily(date)),
        }
    }

    pub fn seeded(seed_text: &str) -> Play {
        Play {
            game: Game::new(Seed::from_text(seed_text)),
        }
    }

    /// `up`, `down`, `left`, `right`, `wait`, `descend` or `quaff`. Returns
    /// whether a turn passed.
    pub fn act(&mut self, action: &str) -> bool {
        Action::from_name(action).is_some_and(|a| self.game.act(a))
    }

    pub fn view(&self) -> String {
        serde_json::to_string(&self.game.view()).expect("views always serialize")
    }

    /// RGBA pixels of the current floor, without fog or creatures.
    #[wasm_bindgen(js_name = floorPixels)]
    pub fn floor_pixels(&self) -> Vec<u8> {
        TILESET
            .with(|t| t.get_or_init(Tileset::builtin).render(self.game.world(), 1))
            .into_raw()
    }

    /// The run so far as a compact replay code.
    pub fn replay(&self) -> String {
        encode(self.game.actions())
    }
}
