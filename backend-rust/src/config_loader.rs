use config::{Config, ConfigError, Environment, File};

pub fn load_config() -> Result<Config, ConfigError> {
    dotenvy::dotenv().ok();

    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
    let env_file = format!("config/{}.toml", app_env);

    Config::builder()
        .add_source(File::with_name("config/default").required(false))
        .add_source(File::with_name(&env_file.trim_end_matches(".toml")).required(false))
        .add_source(Environment::default().separator("_"))
        .build()
}
