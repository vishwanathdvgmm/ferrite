use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub enum BuildError {
    IoError(std::io::Error),
    ManifestMissing,
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildError::IoError(e) => write!(f, "I/O error during build: {}", e)?,
            BuildError::ManifestMissing => {
                write!(f, "ferrite.toml not found in the current directory")?
            }
        };
        Ok(())
    }
}

impl From<std::io::Error> for BuildError {
    fn from(e: std::io::Error) -> Self {
        BuildError::IoError(e)
    }
}

pub fn build_project(dir: &Path) -> Result<(), BuildError> {
    let manifest_path = dir.join("ferrite.toml");
    if !manifest_path.exists() {
        return Err(BuildError::ManifestMissing);
    }

    let target_dir = dir.join("target");
    if !target_dir.exists() {
        fs::create_dir_all(&target_dir)?;
    }

    let src_dir = dir.join("src");
    let mut num_files = 0;

    if src_dir.exists() && src_dir.is_dir() {
        for entry in fs::read_dir(&src_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("fe") {
                num_files += 1;
                // Stub: In a real build system, we would parse and compile the file here.
                // For now, we just pretend to compile it and output a dummy artifact.
                let file_stem = path.file_stem().unwrap().to_str().unwrap();
                let output_path = target_dir.join(format!("{}.ir", file_stem));
                fs::write(output_path, format!("// Compiled IR for {}", file_stem))?;
            }
        }
    }

    println!(
        "✅ Built {} Ferrite source file(s) into target/ directory.",
        num_files
    );
    Ok(())
}

pub fn clean_project(dir: &Path) -> Result<(), BuildError> {
    let target_dir = dir.join("target");
    if target_dir.exists() {
        fs::remove_dir_all(&target_dir)?;
        println!("✅ Cleaned target/ directory.");
    } else {
        println!("target/ directory does not exist, nothing to clean.");
    }
    Ok(())
}
