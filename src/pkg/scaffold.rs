// ── Ferrite Project Scaffolding ─────────────────────────────────────
//
// Implements `ferrite init` and `ferrite new <name>` commands
// for creating new Ferrite projects with standard directory layout.

use std::fs;
use std::path::Path;

use super::manifest::Manifest;

// ── Scaffold Errors ─────────────────────────────────────────────────

#[derive(Debug)]
pub enum ScaffoldError {
    IoError(std::io::Error),
    AlreadyExists(String),
}

impl std::fmt::Display for ScaffoldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScaffoldError::IoError(e) => write!(f, "I/O error: {}", e)?,
            ScaffoldError::AlreadyExists(path) => write!(f, "'{}' already exists", path)?,
        };
        Ok(())
    }
}

impl From<std::io::Error> for ScaffoldError {
    fn from(e: std::io::Error) -> Self {
        ScaffoldError::IoError(e)
    }
}

// ── Default Source Template ─────────────────────────────────────────

const DEFAULT_MAIN_FE: &str = r#"// Welcome to Ferrite!
// Run this program with: ferrite run src/main.fe

fun main() {
    println("Hello, Ferrite!");
}
"#;

const DEFAULT_TEST_FE: &str = r#"// Example test file
// Run with: ferrite run tests/test_main.fe

fun test_addition() -> int {
    keep result: int = 2 + 2;
    return result;
}
"#;

// ── `ferrite init` ──────────────────────────────────────────────────

/// Initialize a Ferrite project in the current working directory.
/// Creates `ferrite.toml` and `src/main.fe` if they don't already exist.
pub fn init_project(dir: &Path) -> Result<(), ScaffoldError> {
    let manifest_path = dir.join("ferrite.toml");

    if manifest_path.exists() {
        return Err(ScaffoldError::AlreadyExists(
            "ferrite.toml (project already initialized)".to_string(),
        ));
    }

    // Derive project name from directory name
    let project_name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("my-project")
        .to_string();

    // Create ferrite.toml
    let toml_content = Manifest::default_toml(&project_name);
    fs::write(&manifest_path, toml_content)?;

    // Create src/ directory and main.fe
    let src_dir = dir.join("src");
    if !src_dir.exists() {
        fs::create_dir_all(&src_dir)?;
    }

    let main_path = src_dir.join("main.fe");
    if !main_path.exists() {
        fs::write(&main_path, DEFAULT_MAIN_FE)?;
    }

    // Create tests/ directory
    let tests_dir = dir.join("tests");
    if !tests_dir.exists() {
        fs::create_dir_all(&tests_dir)?;
    }

    Ok(())
}

// ── `ferrite new <name>` ────────────────────────────────────────────

/// Create a new Ferrite project in a new directory with the given name.
pub fn new_project(parent_dir: &Path, name: &str) -> Result<(), ScaffoldError> {
    let project_dir = parent_dir.join(name);

    if project_dir.exists() {
        return Err(ScaffoldError::AlreadyExists(name.to_string()));
    }

    // Create project directory structure
    fs::create_dir_all(&project_dir)?;
    let src_dir = project_dir.join("src");
    fs::create_dir_all(&src_dir)?;
    let tests_dir = project_dir.join("tests");
    fs::create_dir_all(&tests_dir)?;

    // Generate ferrite.toml
    let toml_content = Manifest::default_toml(name);
    fs::write(project_dir.join("ferrite.toml"), toml_content)?;

    // Generate src/main.fe
    fs::write(src_dir.join("main.fe"), DEFAULT_MAIN_FE)?;

    // Generate tests/test_main.fe
    fs::write(tests_dir.join("test_main.fe"), DEFAULT_TEST_FE)?;

    Ok(())
}

// ── Unit Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    /// Helper: create a unique temporary directory inside the workspace.
    fn temp_dir(suffix: &str) -> std::path::PathBuf {
        let dir = env::current_dir()
            .unwrap()
            .join("target")
            .join("test_scaffold")
            .join(suffix);
        if dir.exists() {
            fs::remove_dir_all(&dir).ok();
        }
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_init_creates_manifest_and_src() {
        let dir = temp_dir("init_basic");
        init_project(&dir).expect("init should succeed");

        assert!(dir.join("ferrite.toml").exists());
        assert!(dir.join("src").join("main.fe").exists());
        assert!(dir.join("tests").exists());

        // Verify manifest is parseable
        let content = fs::read_to_string(dir.join("ferrite.toml")).unwrap();
        let manifest = super::super::manifest::Manifest::from_str(&content);
        assert!(manifest.is_ok());

        // Cleanup
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_init_rejects_existing_project() {
        let dir = temp_dir("init_existing");
        fs::write(dir.join("ferrite.toml"), "# existing").unwrap();

        let result = init_project(&dir);
        assert!(result.is_err());

        // Cleanup
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_new_creates_full_structure() {
        let parent = temp_dir("new_parent");
        new_project(&parent, "hello-ferrite").expect("new should succeed");

        let project = parent.join("hello-ferrite");
        assert!(project.join("ferrite.toml").exists());
        assert!(project.join("src").join("main.fe").exists());
        assert!(project.join("tests").join("test_main.fe").exists());

        // Verify project name in manifest
        let content = fs::read_to_string(project.join("ferrite.toml")).unwrap();
        assert!(content.contains("hello-ferrite"));

        // Cleanup
        fs::remove_dir_all(&parent).ok();
    }

    #[test]
    fn test_new_rejects_existing_dir() {
        let parent = temp_dir("new_dup_parent");
        let project = parent.join("duplicate");
        fs::create_dir_all(&project).unwrap();

        let result = new_project(&parent, "duplicate");
        assert!(result.is_err());

        // Cleanup
        fs::remove_dir_all(&parent).ok();
    }
}
