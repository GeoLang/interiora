//! Venue storage: a map in memory, optionally mirrored to a directory of
//! `IndoorMapDoc` JSON files so a restart keeps its data.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, RwLockReadGuard};

use uuid::Uuid;

use interiora_core::Venue;
use interiora_core::graph::IndoorGraph;
use interiora_core::positioning::PositioningEngine;

use crate::doc::{IndoorMapDoc, build_graph};

/// An uploaded document plus everything derived from it.
pub struct StoredVenue {
    pub doc: IndoorMapDoc,
    /// Absent when the document carried no graph.
    pub graph: Option<IndoorGraph>,
    /// Absent when the document carried no fingerprints.
    pub positioning: Option<PositioningEngine>,
}

impl StoredVenue {
    pub fn build(doc: IndoorMapDoc) -> Result<Self, String> {
        let graph = doc.graph.as_ref().map(build_graph).transpose()?;
        let positioning = (!doc.fingerprints.is_empty()).then(|| {
            let mut engine = PositioningEngine::new();
            engine.load_fingerprints(doc.fingerprints.clone());
            engine
        });
        Ok(Self {
            doc,
            graph,
            positioning,
        })
    }

    pub fn venue(&self) -> &Venue {
        &self.doc.venue
    }
}

/// Shared server state.
#[derive(Clone)]
pub struct AppState {
    venues: Arc<RwLock<HashMap<Uuid, StoredVenue>>>,
    data_dir: Option<PathBuf>,
}

impl AppState {
    /// Build the state, restoring every `*.json` document in `data_dir`. A
    /// document that will not parse or build stops startup rather than
    /// silently dropping a venue.
    pub fn new(data_dir: Option<PathBuf>) -> std::io::Result<Self> {
        let mut venues = HashMap::new();
        if let Some(dir) = &data_dir {
            fs::create_dir_all(dir)?;
            for entry in fs::read_dir(dir)? {
                let path = entry?.path();
                if path.extension().is_some_and(|e| e == "json") {
                    let stored = load(&path)?;
                    venues.insert(stored.venue().id, stored);
                }
            }
        }
        Ok(Self {
            venues: Arc::new(RwLock::new(venues)),
            data_dir,
        })
    }

    pub fn read(&self) -> RwLockReadGuard<'_, HashMap<Uuid, StoredVenue>> {
        self.venues.read().expect("venue store lock poisoned")
    }

    /// Store a document under its venue id, replacing any venue with that id.
    pub fn insert(&self, doc: IndoorMapDoc) -> Result<Uuid, InsertError> {
        let id = doc.venue.id;
        // build before writing, so a document that cannot be used never lands
        // on disk to break the next startup
        let json = serde_json::to_vec_pretty(&doc).map_err(|e| InsertError::Io(e.into()))?;
        let stored = StoredVenue::build(doc).map_err(InsertError::Invalid)?;
        if let Some(dir) = &self.data_dir {
            fs::write(doc_path(dir, id), json).map_err(InsertError::Io)?;
        }
        self.venues
            .write()
            .expect("venue store lock poisoned")
            .insert(id, stored);
        Ok(id)
    }

    /// Remove a venue, returning whether it was there.
    pub fn remove(&self, id: Uuid) -> std::io::Result<bool> {
        let removed = self
            .venues
            .write()
            .expect("venue store lock poisoned")
            .remove(&id)
            .is_some();
        if removed && let Some(dir) = &self.data_dir {
            let path = doc_path(dir, id);
            if path.exists() {
                fs::remove_file(path)?;
            }
        }
        Ok(removed)
    }
}

/// Why an upload could not be stored.
pub enum InsertError {
    /// The document parsed but does not describe a usable venue.
    Invalid(String),
    /// The document could not be written to the data directory.
    Io(std::io::Error),
}

fn doc_path(dir: &Path, id: Uuid) -> PathBuf {
    dir.join(format!("{id}.json"))
}

fn load(path: &Path) -> std::io::Result<StoredVenue> {
    let bytes = fs::read(path)?;
    let doc: IndoorMapDoc = serde_json::from_slice(&bytes)
        .map_err(|e| std::io::Error::other(format!("{}: {e}", path.display())))?;
    StoredVenue::build(doc).map_err(|e| std::io::Error::other(format!("{}: {e}", path.display())))
}
