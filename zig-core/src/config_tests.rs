use super::*;

#[test]
fn default_config_has_local_memory_enabled() {
    let config = ZigConfig::default();
    assert!(config.memory.local);
}

#[test]
fn parse_empty_toml() {
    let config: ZigConfig = toml::from_str("").unwrap();
    assert!(config.memory.local);
}

#[test]
fn parse_memory_local_false() {
    let config: ZigConfig = toml::from_str(
        r#"
[memory]
local = false
"#,
    )
    .unwrap();
    assert!(!config.memory.local);
}

#[test]
fn parse_memory_local_true() {
    let config: ZigConfig = toml::from_str(
        r#"
[memory]
local = true
"#,
    )
    .unwrap();
    assert!(config.memory.local);
}

#[test]
fn parse_partial_config_without_memory_section() {
    let config: ZigConfig = toml::from_str("[other]\nfoo = 1\n").unwrap_or_default();
    assert!(config.memory.local);
}
