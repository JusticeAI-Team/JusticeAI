use std::path::{Path, PathBuf};

use config::{Config, ConfigError, Environment, File};

pub fn load_config() -> Result<Config, ConfigError> {
    let backend_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = backend_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| backend_root.clone());

    load_dotenv_chain(&repo_root, &backend_root);

    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
    let config_dir = backend_root.join("config");
    let env_file = config_dir.join(format!("{}.toml", app_env));

    Config::builder()
        .add_source(File::from(config_dir.join("default.toml")).required(false))
        .add_source(File::from(env_file).required(false))
        .add_source(Environment::default().separator("__"))
        .build()
}

fn load_dotenv_chain(repo_root: &Path, backend_root: &Path) {
    for env_file in [repo_root.join(".env"), backend_root.join(".env")] {
        if env_file.is_file() {
            let _ = dotenvy::from_path(&env_file);
        }
    }
}
