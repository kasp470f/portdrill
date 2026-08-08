use crate::models::Rule;
use std::fs;
use std::path::PathBuf;

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("portdrill")
}

fn config_path() -> PathBuf {
    config_dir().join("rules.json")
}

pub fn load_rules() -> Vec<Rule> {
    let path = config_path();
    if !path.exists() {
        return Vec::new();
    }
    let contents = fs::read_to_string(&path).unwrap_or_default();
    serde_json::from_str(&contents).unwrap_or_default()
}

pub fn save_rules(rules: &[Rule]) -> Result<(), String> {
    let dir = config_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {e}"))?;

    let contents =
        serde_json::to_string_pretty(rules).map_err(|e| format!("Failed to serialize: {e}"))?;

    fs::write(config_path(), contents).map_err(|e| format!("Failed to write config: {e}"))?;

    Ok(())
}
