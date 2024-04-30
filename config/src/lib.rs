#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct Config {
    pub target_function: String,
    pub action: Action,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Default, clap::ValueEnum)]
pub enum Action {
    #[default]
    MeasureStack,
    DoMoreStuff,
}
