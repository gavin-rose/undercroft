use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use undercroft::{Recipe, Registry, STYLES, Seed, World, generate};
use undercroft_render::Tileset;

#[derive(Parser)]
#[command(
    name = "croft",
    version,
    about = "Seeded, validated procedural generation"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate a world from a built-in style or a recipe file
    Gen(GenArgs),
    /// List the built-in styles
    Styles,
    /// List the stages a recipe can use
    Stages,
}

#[derive(Args)]
struct GenArgs {
    /// Built-in style name, or a path to a recipe .toml file
    #[arg(default_value = "dungeon")]
    recipe: String,
    /// Any text. The same seed and recipe always give the same world
    #[arg(long, conflicts_with = "daily")]
    seed: Option<String>,
    /// Use today's shared daily seed (UTC)
    #[arg(long)]
    daily: bool,
    /// Override the recipe's size, e.g. 120x60
    #[arg(long, value_parser = parse_size)]
    size: Option<(i32, i32)>,
    /// Output format. Defaults to png when --out ends in .png, otherwise ascii
    #[arg(long, value_enum)]
    format: Option<Format>,
    /// Write to this file instead of stdout
    #[arg(long, short)]
    out: Option<PathBuf>,
    /// Colour ASCII output using each tag's colour (needs a truecolor terminal)
    #[arg(long)]
    color: bool,
    /// Tileset manifest for png output (defaults to the bundled Kenney 1-Bit Pack)
    #[arg(long)]
    tileset: Option<PathBuf>,
    /// Pixel scale for png output
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..=8))]
    scale: u32,
}

#[derive(Clone, Copy, PartialEq, ValueEnum)]
enum Format {
    Ascii,
    Json,
    Png,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Gen(args) => gen_world(args),
        Command::Styles => {
            for &(name, _) in STYLES {
                let recipe = Recipe::style(name).unwrap();
                println!("{name:<12} {}", recipe.description.unwrap_or_default());
            }
            Ok(())
        }
        Command::Stages => {
            for name in Registry::builtin().names() {
                println!("{name}");
            }
            Ok(())
        }
    }
}

fn gen_world(args: GenArgs) -> Result<()> {
    let mut recipe = load_recipe(&args.recipe)?;
    if let Some((w, h)) = args.size {
        recipe.width = w;
        recipe.height = h;
    }
    let (label, seed) = match (args.daily, args.seed) {
        (true, _) => {
            let date = today_utc();
            (format!("daily {date}"), Seed::daily(&date))
        }
        (false, Some(text)) => {
            let seed = Seed::from_text(&text);
            (text, seed)
        }
        (false, None) => {
            let text = random_seed_text();
            let seed = Seed::from_text(&text);
            (text, seed)
        }
    };

    let format = args.format.unwrap_or_else(|| {
        let is_png = args
            .out
            .as_ref()
            .is_some_and(|p| p.extension().is_some_and(|e| e == "png"));
        if is_png { Format::Png } else { Format::Ascii }
    });
    if format == Format::Png && args.out.is_none() {
        bail!("png output needs --out <FILE>");
    }

    let world = generate(seed, &recipe)?;
    eprintln!("seed: {label}  (attempt {})", world.attempt);

    if format == Format::Png {
        let tileset = match &args.tileset {
            Some(path) => Tileset::load(path)?,
            None => Tileset::builtin(),
        };
        let path = args.out.as_ref().unwrap();
        tileset
            .render(&world, args.scale)
            .save(path)
            .with_context(|| format!("writing {}", path.display()))?;
        eprintln!("wrote {}", path.display());
        return Ok(());
    }

    let sink: Box<dyn Write> = match &args.out {
        Some(path) => {
            Box::new(File::create(path).with_context(|| format!("creating {}", path.display()))?)
        }
        None => Box::new(io::stdout().lock()),
    };
    let mut out = BufWriter::new(sink);
    match format {
        Format::Json => serde_json::to_writer(&mut out, &world)?,
        Format::Ascii if args.color => write_colored(&mut out, &world)?,
        Format::Ascii | Format::Png => write!(out, "{world}")?,
    }
    out.flush()?;
    Ok(())
}

fn load_recipe(name: &str) -> Result<Recipe> {
    if let Some(recipe) = Recipe::style(name) {
        return Ok(recipe);
    }
    let path = Path::new(name);
    if !path.exists() {
        let styles: Vec<&str> = STYLES.iter().map(|(n, _)| *n).collect();
        bail!(
            "no style or file named `{name}` (styles: {})",
            styles.join(", ")
        );
    }
    let source = std::fs::read_to_string(path).with_context(|| format!("reading {name}"))?;
    Recipe::from_toml(&source).with_context(|| format!("loading {name}"))
}

fn write_colored(out: &mut impl Write, world: &World) -> io::Result<()> {
    for row in world.terrain.cells().chunks(world.width() as usize) {
        for &tag in row {
            let def = world.tags.def(tag);
            match def.color {
                Some([r, g, b]) => write!(out, "\x1b[38;2;{r};{g};{b}m{}", def.glyph)?,
                None => write!(out, "\x1b[0m{}", def.glyph)?,
            }
        }
        writeln!(out, "\x1b[0m")?;
    }
    Ok(())
}

fn parse_size(s: &str) -> Result<(i32, i32), String> {
    let (w, h) = s
        .split_once('x')
        .ok_or("expected WIDTHxHEIGHT, e.g. 120x60")?;
    let parse = |v: &str| v.trim().parse::<i32>().map_err(|e| format!("`{v}`: {e}"));
    Ok((parse(w)?, parse(h)?))
}

fn random_seed_text() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", nanos as u64)
}

/// Today's date as `YYYY-MM-DD` in UTC.
fn today_utc() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (y, m, d) = civil_from_days((secs / 86_400) as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Days since 1970-01-01 to a calendar date (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_383), (2025, 10, 22));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
    }
}
