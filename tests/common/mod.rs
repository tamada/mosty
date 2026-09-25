//! Helpers shared by the integration tests.

#![allow(dead_code)] // Each test crate uses a part of the helpers.

use std::path::{Path, PathBuf};

/// Returns the path of the file in `testdata`.
pub fn testdata(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("testdata")
        .join(name)
}

/// A temporary directory removed on drop.
pub struct TempDir(PathBuf);

impl TempDir {
    /// Creates an empty directory unique to the test name and the process.
    pub fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("mosty-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    /// Copies the files in `testdata` into this directory.
    pub fn with_testdata(self, names: &[&str]) -> Self {
        for name in names {
            std::fs::copy(testdata(name), self.0.join(name)).unwrap();
        }
        self
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
