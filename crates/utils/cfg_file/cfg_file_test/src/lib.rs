pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod test_cfg_file {
    use std::collections::HashMap;
    use serde::{Deserialize, Serialize};
    use cfg_file::config::ConfigFile;
    use cfg_file::ConfigFile;

    #[derive(ConfigFile, Deserialize, Serialize, Default)]
    #[cfg_file(path = "./.temp/example_cfg.toml")]
    struct ExampleConfig {
        name: String,
        age: i32,
        hobby: Vec<String>,
        secret: HashMap<String, String>,
    }

    #[tokio::test]
    async fn test_config_file_serialization() {
        let mut example = ExampleConfig {
            name: "Weicao".to_string(),
            age: 22,
            hobby: vec![ "Programming", "Painting" ]
                .iter()
                .map(|m| m.to_string())
                .collect(),
            secret: HashMap::new()
        };
        let secret_no_comments = "Actually, I'm really too lazy to write comments, documentation, and unit tests.";
        example.secret
            .entry("No comments".to_string())
            .insert_entry(secret_no_comments.to_string());

        let secret_peek = "Of course, it's peeking at you who's reading the source code.";
        example.secret
            .entry("Peek".to_string())
            .insert_entry(secret_peek.to_string());

        ExampleConfig::write(&example).await; // Write to default path.

        // Read from default path.
        let read_cfg = ExampleConfig::read().await;
        assert_eq!(read_cfg.name, "Weicao");
        assert_eq!(read_cfg.age, 22);
        assert_eq!(read_cfg.hobby, vec![ "Programming", "Painting" ]);
        assert_eq!(read_cfg.secret["No comments"], secret_no_comments);
        assert_eq!(read_cfg.secret["Peek"], secret_peek);
    }
}