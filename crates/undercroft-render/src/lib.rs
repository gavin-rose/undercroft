//! Renders undercroft worlds to images using a tileset: a sheet of
//! white-on-transparent tiles plus a TOML manifest mapping tags to tiles.
//!
//! ```
//! use undercroft::{Recipe, Seed, generate};
//! use undercroft_render::Tileset;
//!
//! let world = generate(Seed::from_text("hello"), &Recipe::style("dungeon").unwrap()).unwrap();
//! let image = Tileset::builtin().render(&world, 1);
//! assert_eq!(image.width(), 80 * 16);
//! ```

use std::collections::BTreeMap;
use std::path::Path;

use image::{Rgba, RgbaImage, imageops};
use serde::Deserialize;
use undercroft::{Pos, World};

pub use image;

const BUILTIN_MANIFEST: &str = include_str!("../assets/kenney-1bit.toml");
const BUILTIN_SHEET: &[u8] = include_bytes!("../assets/kenney-1bit/tiles.png");

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("couldn't parse tileset manifest: {0}")]
    Manifest(#[from] toml::de::Error),
    #[error("couldn't read {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("couldn't decode tilesheet: {0}")]
    Image(#[from] image::ImageError),
    #[error("tag `{tag}` uses tile [{col}, {row}], which is outside the {cols}x{rows} sheet")]
    TileOutOfRange {
        tag: String,
        col: u32,
        row: u32,
        cols: u32,
        rows: u32,
    },
    #[error("tag `{0}` lists no tiles")]
    NoTiles(String),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    /// Path to the sheet, relative to the manifest.
    image: String,
    tile_size: u32,
    /// Gap between tiles in the sheet, in pixels.
    #[serde(default)]
    spacing: u32,
    background: [u8; 3],
    #[serde(default)]
    tags: BTreeMap<String, TileDef>,
}

/// How one terrain tag is drawn.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TileDef {
    /// Candidate tiles as `[column, row]`. Each cell picks one by hashing its
    /// position, so the choice is stable across renders.
    pub tiles: Vec<[u32; 2]>,
    /// Tint for the tile's pixels. Defaults to the tag's colour.
    #[serde(default)]
    pub fg: Option<[u8; 3]>,
    /// Colour behind the tile. Defaults to the tileset background.
    #[serde(default)]
    pub bg: Option<[u8; 3]>,
    /// Draw only the background when no neighbour is walkable.
    #[serde(default)]
    pub hide_enclosed: bool,
}

pub struct Tileset {
    sheet: RgbaImage,
    tile_size: u32,
    spacing: u32,
    background: [u8; 3],
    tags: BTreeMap<String, TileDef>,
}

impl Tileset {
    /// Kenney's 1-Bit Pack (CC0), bundled with the crate.
    pub fn builtin() -> Self {
        Tileset::from_parts(BUILTIN_MANIFEST, BUILTIN_SHEET).expect("built-in tileset is valid")
    }

    /// Loads a manifest from disk. Its `image` path is relative to the manifest.
    pub fn load(manifest_path: &Path) -> Result<Self, Error> {
        let read = |path: &Path| {
            std::fs::read(path).map_err(|source| Error::Io {
                path: path.display().to_string(),
                source,
            })
        };
        let manifest_bytes = read(manifest_path)?;
        let manifest_src = String::from_utf8_lossy(&manifest_bytes);
        let manifest: Manifest = toml::from_str(&manifest_src)?;
        let dir = manifest_path.parent().unwrap_or(Path::new("."));
        let sheet = read(&dir.join(&manifest.image))?;
        Tileset::build(manifest, &sheet)
    }

    /// Builds a tileset from a manifest's text and the sheet's encoded bytes.
    /// The manifest's `image` field is ignored.
    pub fn from_parts(manifest: &str, sheet: &[u8]) -> Result<Self, Error> {
        Tileset::build(toml::from_str(manifest)?, sheet)
    }

    fn build(manifest: Manifest, sheet: &[u8]) -> Result<Self, Error> {
        let sheet = image::load_from_memory(sheet)?.to_rgba8();
        let pitch = manifest.tile_size + manifest.spacing;
        let cols = (sheet.width() + manifest.spacing) / pitch;
        let rows = (sheet.height() + manifest.spacing) / pitch;
        for (tag, def) in &manifest.tags {
            if def.tiles.is_empty() {
                return Err(Error::NoTiles(tag.clone()));
            }
            for &[col, row] in &def.tiles {
                if col >= cols || row >= rows {
                    let tag = tag.clone();
                    return Err(Error::TileOutOfRange {
                        tag,
                        col,
                        row,
                        cols,
                        rows,
                    });
                }
            }
        }
        Ok(Tileset {
            sheet,
            tile_size: manifest.tile_size,
            spacing: manifest.spacing,
            background: manifest.background,
            tags: manifest.tags,
        })
    }

    pub fn tile_size(&self) -> u32 {
        self.tile_size
    }

    /// Renders the world's terrain, `scale` screen pixels per sheet pixel.
    pub fn render(&self, world: &World, scale: u32) -> RgbaImage {
        let ts = self.tile_size;
        let (w, h) = (world.width() as u32, world.height() as u32);
        let mut out = RgbaImage::new(w * ts, h * ts);
        for (i, &tag) in world.terrain.cells().iter().enumerate() {
            let pos = world.terrain.pos(i);
            let tag_def = world.tags.def(tag);
            let tag_color = tag_def.color.unwrap_or([255, 255, 255]);
            let (x0, y0) = (pos.x as u32 * ts, pos.y as u32 * ts);
            let Some(def) = self.tags.get(world.tags.name(tag)) else {
                // Unknown to this tileset: a solid square in the tag's colour.
                fill(&mut out, x0, y0, ts, tag_color);
                continue;
            };
            let bg = def.bg.unwrap_or(self.background);
            fill(&mut out, x0, y0, ts, bg);
            if def.hide_enclosed && !touches_walkable(world, pos) {
                continue;
            }
            let [col, row] = def.tiles[variant(world.seed, pos, def.tiles.len())];
            let fg = def.fg.unwrap_or(tag_color);
            let pitch = ts + self.spacing;
            for ty in 0..ts {
                for tx in 0..ts {
                    let alpha = self.sheet.get_pixel(col * pitch + tx, row * pitch + ty)[3];
                    if alpha > 0 {
                        let px = out.get_pixel_mut(x0 + tx, y0 + ty);
                        *px = blend(bg, fg, alpha);
                    }
                }
            }
        }
        if scale > 1 {
            imageops::resize(
                &out,
                out.width() * scale,
                out.height() * scale,
                imageops::FilterType::Nearest,
            )
        } else {
            out
        }
    }
}

fn fill(out: &mut RgbaImage, x0: u32, y0: u32, size: u32, [r, g, b]: [u8; 3]) {
    for y in y0..y0 + size {
        for x in x0..x0 + size {
            out.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }
}

fn blend(bg: [u8; 3], fg: [u8; 3], alpha: u8) -> Rgba<u8> {
    let mix =
        |b: u8, f: u8| ((b as u32 * (255 - alpha as u32) + f as u32 * alpha as u32) / 255) as u8;
    Rgba([mix(bg[0], fg[0]), mix(bg[1], fg[1]), mix(bg[2], fg[2]), 255])
}

fn touches_walkable(world: &World, p: Pos) -> bool {
    (-1..=1).any(|dy| {
        (-1..=1).any(|dx| {
            world.walkable(Pos {
                x: p.x + dx,
                y: p.y + dy,
            })
        })
    })
}

/// Stable per position, different between worlds.
fn variant(seed: u64, p: Pos, len: usize) -> usize {
    let mut h = seed
        ^ (p.x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (p.y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    (h % len as u64) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use undercroft::{Recipe, STYLES, Seed, generate};

    #[test]
    fn renders_every_style_at_the_right_size() {
        let tileset = Tileset::builtin();
        for &(name, _) in STYLES {
            let world = generate(Seed::from_text("render"), &Recipe::style(name).unwrap()).unwrap();
            let img = tileset.render(&world, 2);
            assert_eq!(img.width(), world.width() as u32 * 32, "{name}");
            assert_eq!(img.height(), world.height() as u32 * 32, "{name}");
        }
    }

    #[test]
    fn rendering_is_deterministic() {
        let world = generate(
            Seed::from_text("same"),
            &Recipe::style("overworld").unwrap(),
        )
        .unwrap();
        let tileset = Tileset::builtin();
        assert_eq!(tileset.render(&world, 1), tileset.render(&world, 1));
    }

    #[test]
    fn unknown_tags_render_in_their_own_colour() {
        let src = "gen_version = 1\nwidth = 8\nheight = 8\n[tags.lava]\nglyph = \"%\"\ncolor = [230, 80, 20]\n[[stage]]\nuse = \"fill\"\ntag = \"lava\"\n";
        let world = generate(Seed(0), &Recipe::from_toml(src).unwrap()).unwrap();
        let img = Tileset::builtin().render(&world, 1);
        assert_eq!(img.get_pixel(5, 5), &Rgba([230, 80, 20, 255]));
    }

    #[test]
    fn bad_tile_coordinates_are_rejected() {
        let manifest = "image = \"x.png\"\ntile_size = 16\nbackground = [0, 0, 0]\n[tags.wall]\ntiles = [[99, 0]]\n";
        let err = Tileset::from_parts(manifest, BUILTIN_SHEET).err().unwrap();
        assert!(
            matches!(err, Error::TileOutOfRange { col: 99, .. }),
            "{err}"
        );
    }
}
