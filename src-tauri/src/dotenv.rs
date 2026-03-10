use std::fs;
use std::path::Path;

/// Load environment variables from a .env file (does not override existing vars).
pub fn load_dotenv(path: &Path) {
    if !path.exists() {
        return;
    }
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            // setdefault: only set if not already defined
            if std::env::var(key).is_err() {
                std::env::set_var(key, value);
            }
        }
    }
}

/// Load .env files from multiple locations (first found wins per variable):
///   1. CosKit data dir (~/Library/Application Support/CosKit/.env)
///   2. exe directory
///   3. exe parent directory
///   4. $HOME/.env
pub fn load_dotenv_files() {
    // 1. Data dir
    let data_dir = crate::settings::data_dir();
    load_dotenv(&data_dir.join(".env"));

    // 2-3. Exe directory and its parent
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            load_dotenv(&dir.join(".env"));
            if let Some(parent) = dir.parent() {
                load_dotenv(&parent.join(".env"));
            }
        }
    }

    // 4. Home directory
    if let Some(home) = dirs::home_dir() {
        load_dotenv(&home.join(".env"));
    }
}

/// Get an environment variable, returning empty string if not set.
pub fn get_env_var(key: &str) -> String {
    std::env::var(key).unwrap_or_default()
}
