//! Writes texts to files or stdout.

use mosty::{Error, Result};
use std::path::Path;

/// Returns true if the destination means stdout (`-`).
pub fn is_stdout(dest: &Path) -> bool {
    dest == Path::new("-")
}

/// Writes the text to the destination. `None` and `-` mean stdout.
pub fn write(dest: Option<&Path>, text: &str) -> Result<()> {
    match dest.filter(|dest| !is_stdout(dest)) {
        Some(path) => std::fs::write(path, text).map_err(|e| Error::Io(path.to_path_buf(), e)),
        None => {
            print!("{text}");
            Ok(())
        }
    }
}
