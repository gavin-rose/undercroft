use crate::recipe::GEN_VERSION;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("couldn't parse recipe: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("recipe targets gen_version {0}, but this build supports {GEN_VERSION}")]
    UnsupportedVersion(u32),

    #[error("invalid recipe: {0}")]
    InvalidRecipe(String),

    #[error("unknown stage `{0}`")]
    UnknownStage(String),

    #[error("bad parameters for stage `{stage}`: {source}")]
    StageParams {
        stage: String,
        source: toml::de::Error,
    },

    #[error("no valid world after {attempts} attempts (last rejection: {reason})")]
    Exhausted { attempts: u32, reason: String },
}
