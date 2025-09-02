use config::Config;
use serde::Deserialize;

#[derive(Deserialize, Debug, Default)]
pub struct Settings {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_aof_path")]
    pub aof_path: String,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    25500
}

fn default_aof_path() -> String {
    "aof.rdb".to_string()
}

impl Settings {
    pub fn read() -> Self {
        let settings = Config::builder()
            .add_source(config::File::new("config.toml", config::FileFormat::Toml))
            .add_source(config::Environment::with_prefix("APP").separator("_"))
            .build()
            .unwrap();

        settings
            .try_deserialize()
            .expect("Failed to deserialize settings")
    }
}
