//! Marketplace theme packages: full `ThemeDefinition` validation.
//!
//! A theme package carries **no code**: `theme.json` holds a complete theme
//! definition (mirroring the frontend `ThemeDefinition` schema in
//! `apps/desktop/src/customization/theme/schema.ts`) and `theme.css` is an
//! optional supplemental stylesheet. Validation here matches the frontend
//! schema check field-for-field, plus two hardening guards the schema does
//! not cover:
//!
//! - token values must look like colors (no `url(`, no markup, no script
//!   smuggling — tokens are injected as CSS property values);
//! - `theme.css`, when present, must not contain `url(`, `@import`,
//!   `expression(`, `behavior:` or `javascript:` (no remote fetches /
//!   tracking pixels from "just styling").

use crate::{RegistryError, Result};
use serde::{Deserialize, Serialize};
use sonora_plugin::validate_plugin_id;
use std::collections::HashMap;

/// Required token keys — must match `REQUIRED_THEME_TOKENS` in the frontend
/// theme schema exactly.
pub const REQUIRED_THEME_TOKENS: &[&str] = &[
    "--bg-base",
    "--bg-surface",
    "--bg-surface-hover",
    "--bg-elevated",
    "--bg-card",
    "--bg-card-hover",
    "--bg-active",
    "--bg-glass",
    "--accent-primary",
    "--accent-primary-hover",
    "--accent-primary-active",
    "--accent-secondary",
    "--accent-secondary-hover",
    "--accent-glow",
    "--accent-subtle",
    "--text-primary",
    "--text-secondary",
    "--text-muted",
    "--text-disabled",
    "--text-inverse",
    "--border-subtle",
    "--border-medium",
    "--border-focus",
    "--border-active",
    "--state-success",
    "--state-warning",
    "--state-danger",
    "--state-info",
];

/// Maximum supplemental stylesheet size.
pub const MAX_THEME_CSS_BYTES: usize = 256 * 1024;

/// A validated marketplace theme definition (subset of the frontend
/// `ThemeDefinition` that the backend understands).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeDefinition {
    pub id: String,
    pub name: String,
    pub version: semver::Version,
    pub author: String,
    pub mode: ThemeMode,
    #[serde(default)]
    pub description: Option<String>,
    pub tokens: HashMap<String, String>,
}

/// `dark` | `light`, matching the frontend `ThemeMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Dark,
    Light,
}

impl ThemeMode {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ThemeMode::Dark => "dark",
            ThemeMode::Light => "light",
        }
    }
}

/// Parse and validate a `theme.json` document.
pub fn validate_theme_definition(text: &str) -> Result<ThemeDefinition> {
    let raw: serde_json::Value = serde_json::from_str(text).map_err(|e| {
        RegistryError::UnsafePackage("theme".to_string(), format!("theme.json malformed: {e}"))
    })?;
    let obj = raw.as_object().ok_or_else(|| {
        RegistryError::UnsafePackage(
            "theme".to_string(),
            "theme.json must be an object".to_string(),
        )
    })?;

    let get_str = |key: &str| -> Result<String> {
        obj.get(key)
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| {
                RegistryError::UnsafePackage(
                    "theme".to_string(),
                    format!("theme.json lacks string field '{key}'"),
                )
            })
    };

    let id = get_str("id")?;
    validate_plugin_id(&id)
        .map_err(|e| RegistryError::UnsafePackage(id.clone(), format!("bad theme id: {e}")))?;
    let name = get_str("name")?;
    if name.trim().is_empty() || name.len() > 128 {
        return Err(RegistryError::UnsafePackage(
            id,
            "theme name must be 1..=128 characters".to_string(),
        ));
    }
    let version_raw = get_str("version")?;
    let version = semver::Version::parse(&version_raw).map_err(|_| {
        RegistryError::UnsafePackage(id.clone(), "theme version must be semver".to_string())
    })?;
    let author = get_str("author")?;
    if author.trim().is_empty() {
        return Err(RegistryError::UnsafePackage(
            id,
            "theme author must not be empty".to_string(),
        ));
    }
    let mode_raw = get_str("mode")?;
    let mode = match mode_raw.as_str() {
        "dark" => ThemeMode::Dark,
        "light" => ThemeMode::Light,
        _ => {
            return Err(RegistryError::UnsafePackage(
                id,
                "theme mode must be \"dark\" or \"light\"".to_string(),
            ))
        }
    };
    let description = obj
        .get("description")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    if let Some(desc) = &description {
        if desc.len() > 1024 {
            return Err(RegistryError::UnsafePackage(
                id,
                "theme description must be ≤1024 characters".to_string(),
            ));
        }
    }
    let tokens_value = obj
        .get("tokens")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| {
            RegistryError::UnsafePackage(
                id.clone(),
                "theme.json lacks a \"tokens\" object".to_string(),
            )
        })?;
    let mut tokens = HashMap::with_capacity(tokens_value.len());
    for (key, value) in tokens_value {
        let val = value.as_str().ok_or_else(|| {
            RegistryError::UnsafePackage(id.clone(), format!("token '{key}' must be a string"))
        })?;
        validate_token_value(&id, key, val)?;
        tokens.insert(key.clone(), val.to_string());
    }
    for required in REQUIRED_THEME_TOKENS {
        match tokens.get(*required) {
            Some(v) if !v.trim().is_empty() => {}
            _ => {
                return Err(RegistryError::UnsafePackage(
                    id,
                    format!("theme lacks required token '{required}'"),
                ))
            }
        }
    }

    Ok(ThemeDefinition {
        id,
        name,
        version,
        author,
        mode,
        description,
        tokens,
    })
}

/// Token values are injected as CSS property values: allow color-ish text,
/// reject anything that could smuggle markup, scripts, or remote fetches.
fn validate_token_value(id: &str, key: &str, value: &str) -> Result<()> {
    let bad = value.trim().is_empty()
        || value.len() > 256
        || value.contains('<')
        || value.contains('>')
        || value.contains('"')
        || value.contains('\'')
        || value.contains('`')
        || value.contains(';');
    if bad {
        return Err(RegistryError::UnsafePackage(
            id.to_string(),
            format!("token '{key}' has an illegal value"),
        ));
    }
    let lower = value.to_ascii_lowercase();
    for forbidden in [
        "url(",
        "expression(",
        "javascript:",
        "behavior:",
        "@import",
        "data:",
    ] {
        if lower.contains(forbidden) {
            return Err(RegistryError::UnsafePackage(
                id.to_string(),
                format!("token '{key}' must be a plain color value"),
            ));
        }
    }
    Ok(())
}

/// Validate supplemental `theme.css` content (called at install and at serve
/// time — installed files are not trusted).
pub fn validate_theme_css(id: &str, css: &[u8]) -> Result<()> {
    if css.len() > MAX_THEME_CSS_BYTES {
        return Err(RegistryError::UnsafePackage(
            id.to_string(),
            "theme.css exceeds size limit".to_string(),
        ));
    }
    let text = std::str::from_utf8(css).map_err(|_| {
        RegistryError::UnsafePackage(id.to_string(), "theme.css must be UTF-8".to_string())
    })?;
    let lower = text.to_ascii_lowercase();
    for forbidden in ["url(", "@import", "expression(", "behavior:", "javascript:"] {
        if lower.contains(forbidden) {
            return Err(RegistryError::UnsafePackage(
                id.to_string(),
                format!(
                    "theme.css must not contain '{forbidden}' (no remote fetches from styling)"
                ),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_theme_json() -> serde_json::Value {
        let mut tokens = serde_json::Map::new();
        for key in REQUIRED_THEME_TOKENS {
            tokens.insert(key.to_string(), serde_json::json!("#112233"));
        }
        serde_json::json!({
            "id": "org.sonora.theme.test",
            "name": "Test Theme",
            "version": "1.0.0",
            "author": "Tester",
            "mode": "dark",
            "tokens": tokens,
        })
    }

    #[test]
    fn accepts_full_definition() {
        let def = validate_theme_definition(&full_theme_json().to_string()).unwrap();
        assert_eq!(def.mode, ThemeMode::Dark);
        assert_eq!(def.tokens.len(), REQUIRED_THEME_TOKENS.len());
    }

    #[test]
    fn rejects_missing_token_bad_mode_and_injection() {
        let mut v = full_theme_json();
        v["tokens"].as_object_mut().unwrap().remove("--bg-base");
        assert!(validate_theme_definition(&v.to_string()).is_err());

        let mut v = full_theme_json();
        v["mode"] = serde_json::json!("neon");
        assert!(validate_theme_definition(&v.to_string()).is_err());

        let mut v = full_theme_json();
        v["tokens"]["--bg-base"] = serde_json::json!("url(https://evil.test/x.png)");
        assert!(validate_theme_definition(&v.to_string()).is_err());

        let mut v = full_theme_json();
        v["tokens"]["--text-primary"] = serde_json::json!("red; color: expression(x)");
        assert!(validate_theme_definition(&v.to_string()).is_err());
    }

    #[test]
    fn css_guard_rejects_remote_fetch() {
        assert!(validate_theme_css("t", b":root { --x: #fff; }").is_ok());
        assert!(validate_theme_css("t", b"@import url(https://evil.test/a.css);").is_err());
        assert!(
            validate_theme_css("t", b".a { background: URL(https://evil.test/a.png); }").is_err()
        );
    }
}
