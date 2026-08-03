#![cfg(test)]

use std::{
    fs,
    path::{Path, PathBuf},
};

use pastral_domain::BlobObjectId;
use rusqlite::Connection;

pub(crate) struct TestRoot {
    path: PathBuf,
}

impl TestRoot {
    pub(crate) fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("pastral-storage-test-{}", BlobObjectId::new_v4()));
        fs::create_dir_all(&path).expect("create disposable test root");
        Self { path }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub(crate) fn open_test_connection() -> (TestRoot, Connection) {
    let root = TestRoot::new();
    let connection = Connection::open(root.path().join("metadata.sqlite3"))
        .expect("open disposable SQLite database");
    (root, connection)
}
