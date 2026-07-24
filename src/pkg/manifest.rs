// ── Ferrite Package Manifest (`ferrite.toml`) ──────────────────────
//
// A minimal, zero-dependency TOML parser for Ferrite project manifests.
// We intentionally avoid pulling in external crates (like `toml` or `serde`)
// to keep the compiler's dependency tree lean and build times fast.

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;

// ── Data Types ──────────────────────────────────────────────────────

/// Represents a fully parsed `ferrite.toml` manifest.
#[derive(Debug, Clone)]
pub struct Manifest {
    pub package: PackageInfo,
    pub dependencies: HashMap<String, String>,
    pub profiles: HashMap<String, ProfileConfig>,
}

/// The `[package]` table.
#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub authors: Vec<String>,
    pub edition: String,
    pub license: String,
    pub description: String,
}

/// A `[profile.*]` table (e.g., `[profile.debug]`, `[profile.release]`).
#[derive(Debug, Clone)]
pub struct ProfileConfig {
    pub opt_level: u8,
    pub debug: bool,
}

// ── Error Types ─────────────────────────────────────────────────────

#[derive(Debug)]
pub enum ManifestError {
    IoError(std::io::Error),
    ParseError(String),
    MissingField(String),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManifestError::IoError(e) => write!(f, "I/O error reading manifest: {}", e)?,
            ManifestError::ParseError(msg) => write!(f, "Manifest parse error: {}", msg)?,
            ManifestError::MissingField(field) => {
                write!(f, "Missing required field '{}' in [package]", field)?
            }
        };
        Ok(())
    }
}

impl From<std::io::Error> for ManifestError {
    fn from(e: std::io::Error) -> Self {
        ManifestError::IoError(e)
    }
}

// ── TOML Value Representation ───────────────────────────────────────

/// A minimal TOML value type sufficient for ferrite.toml parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum TomlValue {
    String(String),
    Integer(i64),
    Boolean(bool),
    Array(Vec<TomlValue>),
    Table(HashMap<String, TomlValue>),
}

// ── Minimal TOML Parser ─────────────────────────────────────────────

/// Parses a TOML string into a flat map of tables, each containing key-value pairs.
/// Supports: strings, integers, booleans, arrays of strings, and nested dotted tables.
pub fn parse_toml(input: &str) -> Result<HashMap<String, TomlValue>, ManifestError> {
    let mut root: HashMap<String, TomlValue> = HashMap::new();
    let mut current_table: Option<String> = None;

    for (line_num, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Table header: [table] or [table.subtable]
        if line.starts_with('[') && !line.starts_with("[[") {
            let header = line.trim_start_matches('[').trim_end_matches(']').trim();
            if header.is_empty() {
                return Err(ManifestError::ParseError(format!(
                    "Empty table header on line {}",
                    line_num + 1
                )));
            }
            current_table = Some(header.to_string());

            // Ensure the table exists in root as a Table
            ensure_table_path(&mut root, header);
            continue;
        }

        // Key = Value pair
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().to_string();
            let val_str = line[eq_pos + 1..].trim();

            if key.is_empty() {
                return Err(ManifestError::ParseError(format!(
                    "Empty key on line {}",
                    line_num + 1
                )));
            }

            let value = parse_value(val_str, line_num)?;

            if let Some(ref table_name) = current_table {
                // Insert into the current table
                insert_into_table(&mut root, table_name, key, value);
            } else {
                // Top-level key (rare but valid)
                root.insert(key, value);
            }
        }
    }

    Ok(root)
}

/// Ensures a dotted table path like "profile.debug" exists as nested Tables.
fn ensure_table_path(root: &mut HashMap<String, TomlValue>, path: &str) {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = root;

    for part in &parts {
        let key = part.to_string();
        if !current.contains_key(&key) {
            current.insert(key.clone(), TomlValue::Table(HashMap::new()));
        }
        // Navigate into the next level
        let next = current.get_mut(&key).unwrap();
        match next {
            TomlValue::Table(ref mut inner) => {
                current = inner;
            }
            _ => return, // If it's not a table, we can't go deeper
        }
    }
}

/// Inserts a key-value pair into a dotted table path.
fn insert_into_table(
    root: &mut HashMap<String, TomlValue>,
    table_path: &str,
    key: String,
    value: TomlValue,
) {
    let parts: Vec<&str> = table_path.split('.').collect();
    let mut current = root;

    for part in &parts {
        let k = part.to_string();
        let next = current.get_mut(&k).unwrap();
        match next {
            TomlValue::Table(ref mut inner) => {
                current = inner;
            }
            _ => return,
        }
    }

    current.insert(key, value);
}

/// Parses a single TOML value from a string.
fn parse_value(s: &str, line_num: usize) -> Result<TomlValue, ManifestError> {
    let s = s.trim();

    // Strip inline comments (but not inside strings)
    let effective = if s.starts_with('"') || s.starts_with('[') {
        s.to_string()
    } else if let Some(hash_pos) = s.find('#') {
        s[..hash_pos].trim().to_string()
    } else {
        s.to_string()
    };
    let s = effective.as_str();

    // String: "..."
    if s.starts_with('"') {
        let end = s[1..].find('"').ok_or_else(|| {
            ManifestError::ParseError(format!("Unterminated string on line {}", line_num + 1))
        })?;
        return Ok(TomlValue::String(s[1..1 + end].to_string()));
    }

    // Boolean
    if s == "true" {
        return Ok(TomlValue::Boolean(true));
    }
    if s == "false" {
        return Ok(TomlValue::Boolean(false));
    }

    // Integer
    if let Ok(n) = s.parse::<i64>() {
        return Ok(TomlValue::Integer(n));
    }

    // Array: [val, val, ...]
    if s.starts_with('[') && s.ends_with(']') {
        let inner = s[1..s.len() - 1].trim();
        if inner.is_empty() {
            return Ok(TomlValue::Array(vec![]));
        }

        let mut items = Vec::new();
        // Simple split by comma — works for arrays of strings and integers
        for item in split_array_items(inner) {
            items.push(parse_value(item.trim(), line_num)?);
        }
        return Ok(TomlValue::Array(items));
    }

    Err(ManifestError::ParseError(format!(
        "Cannot parse value '{}' on line {}",
        s,
        line_num + 1
    )))
}

/// Splits array items by commas, respecting quoted strings.
fn split_array_items(s: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut in_string = false;

    for (i, ch) in s.char_indices() {
        match ch {
            '"' => in_string = !in_string,
            ',' if !in_string => {
                result.push(s[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }

    let remainder = s[start..].trim();
    if !remainder.is_empty() {
        result.push(remainder);
    }

    result
}

// ── Manifest Loading ────────────────────────────────────────────────

impl Manifest {
    /// Load and parse a `ferrite.toml` file from the given directory path.
    pub fn load(dir: &Path) -> Result<Manifest, ManifestError> {
        let manifest_path = dir.join("ferrite.toml");
        let content = fs::read_to_string(&manifest_path)?;
        Self::from_str(&content)
    }

    /// Parse a manifest from a raw TOML string.
    pub fn from_str(content: &str) -> Result<Manifest, ManifestError> {
        let root = parse_toml(content)?;

        // ── [package] ───────────────────────────────────────────
        let pkg_table = match root.get("package") {
            Some(TomlValue::Table(t)) => t,
            _ => return Err(ManifestError::MissingField("package".into())),
        };

        let name = get_string(pkg_table, "name")?;
        let version = get_string(pkg_table, "version")?;
        let edition = get_string_or(pkg_table, "edition", "2026");
        let license = get_string_or(pkg_table, "license", "MIT");
        let description = get_string_or(pkg_table, "description", "");

        let authors = match pkg_table.get("authors") {
            Some(TomlValue::Array(arr)) => arr
                .iter()
                .filter_map(|v| match v {
                    TomlValue::String(s) => Some(s.clone()),
                    _ => None,
                })
                .collect(),
            _ => vec![],
        };

        let package = PackageInfo {
            name,
            version,
            authors,
            edition,
            license,
            description,
        };

        // ── [dependencies] ──────────────────────────────────────
        let dependencies = match root.get("dependencies") {
            Some(TomlValue::Table(t)) => t
                .iter()
                .filter_map(|(k, v)| match v {
                    TomlValue::String(s) => Some((k.clone(), s.clone())),
                    _ => None,
                })
                .collect(),
            _ => HashMap::new(),
        };

        // ── [profile.*] ────────────────────────────────────────
        let mut profiles = HashMap::new();
        if let Some(TomlValue::Table(profile_table)) = root.get("profile") {
            for (profile_name, profile_val) in profile_table {
                if let TomlValue::Table(cfg) = profile_val {
                    let opt_level = match cfg.get("opt-level") {
                        Some(TomlValue::Integer(n)) => *n as u8,
                        _ => 0,
                    };
                    let debug = match cfg.get("debug") {
                        Some(TomlValue::Boolean(b)) => *b,
                        _ => profile_name == "debug",
                    };
                    profiles.insert(profile_name.clone(), ProfileConfig { opt_level, debug });
                }
            }
        }

        Ok(Manifest {
            package,
            dependencies,
            profiles,
        })
    }

    /// Generate a default `ferrite.toml` string for a new project.
    pub fn default_toml(project_name: &str) -> String {
        let s = format!(
            r#"[package]
name = "{}"
version = "0.1.0"
authors = []
edition = "2026"
license = "MIT"
description = ""

[dependencies]

[profile.debug]
opt-level = 0
debug = true

[profile.release]
opt-level = 3
debug = false
"#,
            project_name
        );
        s
    }
}

// ── Helper Functions ────────────────────────────────────────────────

fn get_string(table: &HashMap<String, TomlValue>, key: &str) -> Result<String, ManifestError> {
    match table.get(key) {
        Some(TomlValue::String(s)) => Ok(s.clone()),
        _ => Err(ManifestError::MissingField(key.into())),
    }
}

fn get_string_or(table: &HashMap<String, TomlValue>, key: &str, default: &str) -> String {
    match table.get(key) {
        Some(TomlValue::String(s)) => s.clone(),
        _ => default.to_string(),
    }
}

// ── Unit Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_manifest() {
        let toml = r#"
[package]
name = "my-project"
version = "1.0.0"
authors = ["Alice", "Bob"]
edition = "2026"
license = "MIT"
description = "A sample Ferrite project"

[dependencies]
ferrite-math = "0.2.0"
ferrite-nn = "0.1.0"

[profile.debug]
opt-level = 0
debug = true

[profile.release]
opt-level = 3
debug = false
"#;
        let manifest = Manifest::from_str(toml).expect("Should parse valid manifest");
        assert_eq!(manifest.package.name, "my-project");
        assert_eq!(manifest.package.version, "1.0.0");
        assert_eq!(manifest.package.authors, vec!["Alice", "Bob"]);
        assert_eq!(manifest.package.edition, "2026");
        assert_eq!(manifest.package.license, "MIT");
        assert_eq!(manifest.package.description, "A sample Ferrite project");
        assert_eq!(manifest.dependencies.len(), 2);
        assert_eq!(
            manifest.dependencies.get("ferrite-math"),
            Some(&"0.2.0".to_string())
        );
        assert_eq!(manifest.profiles.len(), 2);
        assert_eq!(manifest.profiles["debug"].opt_level, 0);
        assert!(manifest.profiles["debug"].debug);
        assert_eq!(manifest.profiles["release"].opt_level, 3);
        assert!(!manifest.profiles["release"].debug);
    }

    #[test]
    fn test_missing_package_name() {
        let toml = r#"
[package]
version = "1.0.0"
"#;
        let result = Manifest::from_str(toml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            format!("{}", err).contains("name"),
            "Error should mention missing 'name' field"
        );
    }

    #[test]
    fn test_missing_package_version() {
        let toml = r#"
[package]
name = "test"
"#;
        let result = Manifest::from_str(toml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            format!("{}", err).contains("version"),
            "Error should mention missing 'version' field"
        );
    }

    #[test]
    fn test_missing_package_table() {
        let toml = r#"
[dependencies]
some-lib = "1.0.0"
"#;
        let result = Manifest::from_str(toml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            format!("{}", err).contains("package"),
            "Error should mention missing [package] table"
        );
    }

    #[test]
    fn test_empty_dependencies() {
        let toml = r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
"#;
        let manifest = Manifest::from_str(toml).expect("Should parse with empty deps");
        assert!(manifest.dependencies.is_empty());
    }

    #[test]
    fn test_default_toml_generation() {
        let toml_str = Manifest::default_toml("hello-world");
        let manifest = Manifest::from_str(&toml_str).expect("Default TOML should be parseable");
        assert_eq!(manifest.package.name, "hello-world");
        assert_eq!(manifest.package.version, "0.1.0");
    }

    #[test]
    fn test_comments_are_ignored() {
        let toml = r#"
# This is a comment
[package]
name = "test" # inline comment
version = "0.1.0"
"#;
        let manifest = Manifest::from_str(toml).expect("Should ignore comments");
        assert_eq!(manifest.package.name, "test");
    }

    #[test]
    fn test_empty_array() {
        let toml = r#"
[package]
name = "test"
version = "0.1.0"
authors = []
"#;
        let manifest = Manifest::from_str(toml).expect("Should parse empty array");
        assert!(manifest.package.authors.is_empty());
    }

    #[test]
    fn test_unterminated_string() {
        let toml = r#"
[package]
name = "unterminated
version = "0.1.0"
"#;
        let result = Manifest::from_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_profile_defaults() {
        let toml = r#"
[package]
name = "test"
version = "0.1.0"
"#;
        let manifest = Manifest::from_str(toml).expect("Should parse without profiles");
        assert!(manifest.profiles.is_empty());
    }
}
