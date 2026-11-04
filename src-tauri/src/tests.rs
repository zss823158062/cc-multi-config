#[cfg(test)]
mod tests {
    use crate::paths::sanitize_slug;
    use crate::settings_merger::{clean_managed_fields, merge_node_to_settings, generate_preview_diff};
    use crate::models::{RelayNode, NodeModels};

    #[test]
    fn test_slug_sanitization() {
        assert_eq!(sanitize_slug("Opus 3.5").unwrap(), "opus-3-5");
        assert_eq!(sanitize_slug("Gemini-3-Pro").unwrap(), "gemini-3-pro");
        assert_eq!(sanitize_slug("My__Custom--Name  ").unwrap(), "my-custom-name");

        // Windows reserved names
        assert!(sanitize_slug("con").is_err());
        assert!(sanitize_slug("prn").is_err());
        assert!(sanitize_slug("aux").is_err());
        assert!(sanitize_slug("nul").is_err());
        assert!(sanitize_slug("com1").is_err());
        assert!(sanitize_slug("lpt1").is_err());

        // Empty
        assert!(sanitize_slug("   ").is_err());
        assert!(sanitize_slug("???").is_err());
    }

    #[test]
    fn test_clean_managed_fields_preserves_user_settings() {
        let base_json = r#"{
            "env": {
                "ANTHROPIC_BASE_URL": "https://old.example.com",
                "ANTHROPIC_AUTH_TOKEN": "secret-token",
                "CUSTOM_ENV_VAR": "keep_this_user_value"
            },
            "permissions": {
                "allow": ["read", "write"]
            },
            "hooks": {
                "preCommit": "echo test"
            }
        }"#;

        let cleaned = clean_managed_fields(base_json).unwrap();
        let env_obj = cleaned["env"].as_object().unwrap();

        // Whitelisted fields removed
        assert!(!env_obj.contains_key("ANTHROPIC_BASE_URL"));
        assert!(!env_obj.contains_key("ANTHROPIC_AUTH_TOKEN"));

        // User custom fields preserved!
        assert_eq!(env_obj["CUSTOM_ENV_VAR"], "keep_this_user_value");
        assert_eq!(cleaned["permissions"]["allow"][0], "read");
        assert_eq!(cleaned["hooks"]["preCommit"], "echo test");
    }

    #[test]
    fn test_merge_node_to_settings() {
        let node = RelayNode {
            id: "node-a".to_string(),
            name: "中转站 A".to_string(),
            base_url: "https://api-a.example.com".to_string(),
            auth_mode: "authToken".to_string(),
            auth_token: "sk-node-a-secret".to_string(),
            models: NodeModels {
                opus: Some("relay-opus-model-id".to_string()),
                sonnet: Some("relay-sonnet-model-id".to_string()),
                haiku: Some("relay-haiku-model-id".to_string()),
                default_model: Some("relay-opus-model-id".to_string()),
            },
            custom_env: None,
            group_id: Some("opus".to_string()),
        };

        let initial_json = r#"{"permissions": {"allow": []}}"#;
        let merged_str = merge_node_to_settings(initial_json, &node, "opus").unwrap();
        let merged: serde_json::Value = serde_json::from_str(&merged_str).unwrap();

        assert_eq!(merged["env"]["ANTHROPIC_BASE_URL"], "https://api-a.example.com");
        assert_eq!(merged["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-node-a-secret");
        assert_eq!(merged["env"]["ANTHROPIC_DEFAULT_OPUS_MODEL"], "relay-opus-model-id");
        assert_eq!(merged["env"]["ANTHROPIC_DEFAULT_SONNET_MODEL"], "relay-sonnet-model-id");
        assert_eq!(merged["env"]["ANTHROPIC_DEFAULT_HAIKU_MODEL"], "relay-haiku-model-id");
        assert_eq!(merged["env"]["ANTHROPIC_MODEL"], "relay-opus-model-id");

        // User permissions intact
        assert!(merged["permissions"]["allow"].is_array());
    }

    #[test]
    fn test_preview_diff_masks_token() {
        let node = RelayNode {
            id: "node-b".to_string(),
            name: "中转站 B".to_string(),
            base_url: "https://api-b.example.com".to_string(),
            auth_mode: "authToken".to_string(),
            auth_token: "secret-new-token".to_string(),
            models: NodeModels::default(),
            custom_env: None,
            group_id: Some("sonnet".to_string()),
        };

        let current = r#"{"env": {"ANTHROPIC_AUTH_TOKEN": "secret-old-token"}}"#;
        let diff = generate_preview_diff("opus", current, &node, "/path/to/my-opus.json").unwrap();

        // Ensure token is masked with asterisks in diff summary
        let summary_text = diff.diff_summary.join("\n");
        assert!(!summary_text.contains("secret-new-token"));
        assert!(!summary_text.contains("secret-old-token"));
        assert!(summary_text.contains("******"));
    }
}
