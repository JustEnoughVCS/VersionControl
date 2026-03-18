use std::{collections::HashMap, path::PathBuf};

use sheet_system::index_source::IndexSource;

#[derive(Default)]
pub struct IndexesTransfer {
    inner: HashMap<PathBuf, IndexSource>,
}

impl IndexesTransfer {
    /// Insert an index file path and its source into the collection
    pub fn insert(&mut self, path: PathBuf, source: IndexSource) -> Option<IndexSource> {
        self.inner.insert(path, source)
    }

    /// Returns an iterator over the index file paths and their sources
    pub fn iter(&self) -> impl Iterator<Item = (&PathBuf, &IndexSource)> {
        self.inner.iter()
    }

    /// Insert an index file path and its source into the collection and return self for chaining
    pub fn with(mut self, path: PathBuf, source: IndexSource) -> Self {
        self.inner.insert(path, source);
        self
    }
}
