use std::collections::BTreeMap;

use serde::de::DeserializeOwned;

use crate::error::Error;
use crate::recipe::{Recipe, StageSpec};
use crate::rng::Rng;
use crate::seed::Seed;
use crate::world::World;

/// How many times generation retries after a stage rejects the world.
pub const MAX_ATTEMPTS: u32 = 64;

/// One step of a recipe. Implement this (plus `Deserialize` for its
/// parameters) and add it to a [`Registry`] to make a custom stage.
pub trait Stage {
    fn run(&self, world: &mut World, rng: &mut Rng) -> Result<(), Fail>;
}

/// Why a stage stopped.
#[derive(Debug, Clone, PartialEq)]
pub enum Fail {
    /// This roll didn't work out (say, stairs too close). Generation retries
    /// with the next attempt's seed.
    Reject(String),
    /// The recipe itself is wrong. Generation stops.
    Invalid(String),
}

impl Fail {
    pub fn reject(reason: impl Into<String>) -> Self {
        Fail::Reject(reason.into())
    }

    pub fn invalid(reason: impl Into<String>) -> Self {
        Fail::Invalid(reason.into())
    }
}

type Maker = fn(toml::Value) -> Result<Box<dyn Stage>, toml::de::Error>;

/// Maps stage names (the `use = "..."` in a recipe) to implementations.
pub struct Registry {
    makers: BTreeMap<String, Maker>,
}

impl Registry {
    pub fn empty() -> Self {
        Registry {
            makers: BTreeMap::new(),
        }
    }

    pub fn builtin() -> Self {
        let mut registry = Registry::empty();
        crate::stages::register_builtins(&mut registry);
        registry
    }

    pub fn register<T: Stage + DeserializeOwned + 'static>(&mut self, name: &str) {
        self.makers.insert(name.to_owned(), make::<T>);
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.makers.keys().map(String::as_str)
    }

    fn build(&self, spec: &StageSpec) -> Result<Box<dyn Stage>, Error> {
        let make = self
            .makers
            .get(&spec.kind)
            .ok_or_else(|| Error::UnknownStage(spec.kind.clone()))?;
        make(toml::Value::Table(spec.params.clone())).map_err(|source| Error::StageParams {
            stage: spec.kind.clone(),
            source,
        })
    }
}

fn make<T: Stage + DeserializeOwned + 'static>(
    params: toml::Value,
) -> Result<Box<dyn Stage>, toml::de::Error> {
    Ok(Box::new(params.try_into::<T>()?))
}

/// Runs `recipe` with the built-in stages.
pub fn generate(seed: Seed, recipe: &Recipe) -> Result<World, Error> {
    generate_with(&Registry::builtin(), seed, recipe)
}

/// Runs `recipe` with a custom set of stages.
pub fn generate_with(registry: &Registry, seed: Seed, recipe: &Recipe) -> Result<World, Error> {
    recipe.validate()?;

    // Seed stages by id or name, not position, so inserting one doesn't reshuffle the rest.
    let mut counts: BTreeMap<&str, u32> = BTreeMap::new();
    let mut stages = Vec::with_capacity(recipe.stages.len());
    for spec in &recipe.stages {
        let n = counts.entry(spec.kind.as_str()).or_insert(0);
        let key = spec
            .id
            .clone()
            .unwrap_or_else(|| format!("{}#{n}", spec.kind));
        *n += 1;
        stages.push((key, registry.build(spec)?));
    }

    let tags = recipe.tag_set();
    let mut last_reason = String::new();
    'attempts: for attempt in 0..MAX_ATTEMPTS {
        let attempt_seed = seed.derive(&format!("attempt:{attempt}"));
        let mut world = World::new(seed.0, attempt, recipe.width, recipe.height, tags.clone());
        for (key, stage) in &stages {
            let mut rng = Rng::new(attempt_seed.derive(key).0);
            match stage.run(&mut world, &mut rng) {
                Ok(()) => {}
                Err(Fail::Reject(reason)) => {
                    last_reason = format!("{key}: {reason}");
                    continue 'attempts;
                }
                Err(Fail::Invalid(reason)) => {
                    return Err(Error::InvalidRecipe(format!("{key}: {reason}")));
                }
            }
        }
        return Ok(world);
    }
    Err(Error::Exhausted {
        attempts: MAX_ATTEMPTS,
        reason: last_reason,
    })
}
