use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use zlf_config::{VectorIndexEngine, ZlfConfig};
use zlf_query::{VectorIndexStrategy, ZlfDatabase, ZlfDatabaseOptions};

pub(crate) struct AppState {
    pub(crate) db_path: RwLock<String>,
    pub(crate) db: RwLock<Option<Arc<ZlfDatabase>>>,
}

impl AppState {
    pub(crate) fn empty() -> Self {
        Self {
            db_path: RwLock::new(String::new()),
            db: RwLock::new(None),
        }
    }
}

pub(crate) async fn ensure_db(state: &AppState, path: &str) -> Result<Arc<ZlfDatabase>, String> {
    // Check if we already have this database open
    {
        let db_path = state.db_path.read().await;
        let db = state.db.read().await;

        if *db_path == path && db.is_some() {
            return Ok(Arc::clone(db.as_ref().unwrap()));
        }
    }

    // Need to open new database - acquire write lock
    let mut db_path = state.db_path.write().await;
    let mut db = state.db.write().await;

    // Double-check after acquiring write lock
    if *db_path == path && db.is_some() {
        return Ok(Arc::clone(db.as_ref().unwrap()));
    }

    // Open database
    let db_path_std = std::path::Path::new(path);
    let options = database_options(&ZlfConfig::load());
    let planner = if db_path_std.exists() {
        ZlfDatabase::open_existing_with_options(db_path_std, options)
    } else {
        std::fs::create_dir_all(db_path_std).map_err(|e| e.to_string())?;
        ZlfDatabase::open_with_options(db_path_std, options)
    }
    .map_err(|e| e.to_string())?;

    let planner = Arc::new(planner);

    // Update state
    *db_path = path.to_string();
    *db = Some(Arc::clone(&planner));

    Ok(planner)
}

pub(crate) fn database_options(config: &ZlfConfig) -> ZlfDatabaseOptions {
    let vector_index = if !config.embedding.enabled {
        VectorIndexStrategy::Disabled
    } else {
        match config.embedding.index_engine {
            VectorIndexEngine::Exact => VectorIndexStrategy::Exact,
            VectorIndexEngine::Hnsw => VectorIndexStrategy::hnsw(),
        }
    };
    ZlfDatabaseOptions { vector_index }
}
