use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Default, Debug, Clone, Copy, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum IntroBanner {
    #[default]
    Long,
    Short,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PrettyPrintMode {
    Always,
    Never,
    Auto,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ExchangeRateFetchingPolicy {
    /// Always fetch exchange rates in the background when the application is started
    #[default]
    OnStartup,

    /// Fetch exchange rates when a currency symbol is used
    OnFirstUse,

    /// Never fetch exchange rates
    Never,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default, deny_unknown_fields)]
#[derive(Default)]
pub struct ExchangeRateConfig {
    pub fetching_policy: ExchangeRateFetchingPolicy,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default, deny_unknown_fields)]
pub struct Config {
    pub intro_banner: IntroBanner,
    pub prompt: String,
    pub pretty_print: PrettyPrintMode,
    pub enter_repl: bool,

    #[serde(skip_serializing)]
    pub load_prelude: bool,

    #[serde(skip_serializing)]
    pub load_user_init: bool,
    pub exchange_rates: ExchangeRateConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prompt: ">>> ".to_owned(),
            intro_banner: IntroBanner::default(),
            pretty_print: PrettyPrintMode::Auto,
            load_prelude: true,
            load_user_init: true,
            exchange_rates: Default::default(),
            enter_repl: true,
        }
    }
}
