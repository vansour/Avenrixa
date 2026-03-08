#[test]
fn test_config_loaded() {
    // 测试 dx.toml 配置文件存在且可解析
    let config_path = std::path::Path::new("dx.toml");
    assert!(config_path.exists(), "dx.toml should exist at dx.toml");

    let content = std::fs::read_to_string(config_path)
        .expect("Should be able to read dx.toml");

    // 验证基本配置项存在
    assert!(content.contains("application"), "Config should have [application] section");
    assert!(content.contains("name"), "Config should have name field");
}
