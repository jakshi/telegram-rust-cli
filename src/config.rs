use serde::Deserialize;
use serde::de::{self, Deserializer};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub telegram: TelegramConfig,
    pub channel: Option<ChannelConfig>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramConfig {
    #[serde(deserialize_with = "deserialize_api_id")]
    pub api_id: i32,
    pub api_hash: String,
    #[serde(default)]
    pub tdlib_log_verbosity: Option<i32>,
}

fn deserialize_api_id<'de, D: Deserializer<'de>>(deserializer: D) -> Result<i32, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrInt {
        Int(i32),
        Str(String),
    }

    match StringOrInt::deserialize(deserializer)? {
        StringOrInt::Int(v) => Ok(v),
        StringOrInt::Str(s) => s.parse::<i32>().map_err(de::Error::custom),
    }
}

#[derive(Debug, Deserialize)]
pub struct ChannelConfig {
    pub chat: String,
    pub message_limit: Option<i32>,
}

impl Config {
    #[cfg(test)]
    pub fn parse(content: &str) -> Result<Self, String> {
        toml::from_str(content).map_err(|e| format!("Failed to parse config: {e}"))
    }

    pub fn load() -> Result<(Self, PathBuf), String> {
        let paths = [
            dirs::home_dir().map(|p| p.join(".config").join("tgrc").join("config.toml")),
            dirs::config_dir().map(|p| p.join("tgrc").join("config.toml")),
            Some(PathBuf::from("config.toml")),
        ];

        for path in paths.into_iter().flatten() {
            if path.exists() {
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
                let config: Config = toml::from_str(&content)
                    .map_err(|e| format!("Failed to parse {}: {e}", path.display()))?;
                return Ok((config, path));
            }
        }

        Err(
            "Config file not found. Create ~/.config/tgrc/config.toml or ./config.toml\n\
             See config.example.toml for the required format."
                .to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_config_with_integer_api_id() {
        let config = Config::parse(
            r#"
            [telegram]
            api_id = 12345678
            api_hash = "abc123"
            "#,
        )
        .unwrap();
        assert_eq!(config.telegram.api_id, 12345678);
        assert_eq!(config.telegram.api_hash, "abc123");
        assert!(config.channel.is_none());
    }

    #[test]
    fn parse_valid_config_with_string_api_id() {
        let config = Config::parse(
            r#"
            [telegram]
            api_id = "12345678"
            api_hash = "abc123"
            "#,
        )
        .unwrap();
        assert_eq!(config.telegram.api_id, 12345678);
    }

    #[test]
    fn parse_config_with_channel_section() {
        let config = Config::parse(
            r#"
            [telegram]
            api_id = 123
            api_hash = "hash"

            [channel]
            chat = "durov"
            message_limit = 10
            "#,
        )
        .unwrap();
        let channel = config.channel.unwrap();
        assert_eq!(channel.chat, "durov");
        assert_eq!(channel.message_limit, Some(10));
    }

    #[test]
    fn parse_config_channel_optional() {
        let config = Config::parse(
            r#"
            [telegram]
            api_id = 123
            api_hash = "hash"
            "#,
        )
        .unwrap();
        assert!(config.channel.is_none());
    }

    #[test]
    fn parse_config_message_limit_optional() {
        let config = Config::parse(
            r#"
            [telegram]
            api_id = 123
            api_hash = "hash"

            [channel]
            chat = "saved"
            "#,
        )
        .unwrap();
        assert_eq!(config.channel.unwrap().message_limit, None);
    }

    #[test]
    fn parse_config_tdlib_log_verbosity() {
        let config = Config::parse(
            r#"
            [telegram]
            api_id = 123
            api_hash = "hash"
            tdlib_log_verbosity = 3
            "#,
        )
        .unwrap();
        assert_eq!(config.telegram.tdlib_log_verbosity, Some(3));
    }

    #[test]
    fn parse_config_tdlib_log_verbosity_defaults_to_none() {
        let config = Config::parse(
            r#"
            [telegram]
            api_id = 123
            api_hash = "hash"
            "#,
        )
        .unwrap();
        assert_eq!(config.telegram.tdlib_log_verbosity, None);
    }

    #[test]
    fn parse_config_missing_api_id_fails() {
        let result = Config::parse(
            r#"
            [telegram]
            api_hash = "hash"
            "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn parse_config_missing_api_hash_fails() {
        let result = Config::parse(
            r#"
            [telegram]
            api_id = 123
            "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn parse_config_invalid_string_api_id_fails() {
        let result = Config::parse(
            r#"
            [telegram]
            api_id = "not_a_number"
            api_hash = "hash"
            "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn parse_config_missing_chat_in_channel_fails() {
        let result = Config::parse(
            r#"
            [telegram]
            api_id = 123
            api_hash = "hash"

            [channel]
            message_limit = 10
            "#,
        );
        assert!(result.is_err());
    }
}
