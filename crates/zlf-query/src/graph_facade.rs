use zlf_core::{Edge, Node, Result, ZlfError};

use crate::ZlfDatabase;

impl ZlfDatabase {
    pub fn get_all_nodes(&self) -> Result<Vec<Node>> {
        self.storage
            .scan_prefix("node:")?
            .into_iter()
            .map(|(_, value)| {
                bincode::deserialize(&value)
                    .map_err(|error| ZlfError::Serialization(error.to_string()))
            })
            .collect()
    }

    pub fn get_all_edges(&self) -> Result<Vec<Edge>> {
        self.storage
            .scan_prefix("edge:")?
            .into_iter()
            .map(|(_, value)| {
                bincode::deserialize(&value)
                    .map_err(|error| ZlfError::Serialization(error.to_string()))
            })
            .collect()
    }
}
