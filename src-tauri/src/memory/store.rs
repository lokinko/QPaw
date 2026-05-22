use std::sync::Arc;

use crate::error::QPawResult;
use crate::models::{
    LayeredMemoryItem, MemoryL0, MemoryL1Concept, MemoryL1Relation, MemoryL2Event,
    MemoryL3Reflection, MemoryLayerFilter,
};
use crate::storage::DocumentStore;

pub struct MemoryStore {
    store: Arc<DocumentStore>,
}

impl MemoryStore {
    pub fn new(store: Arc<DocumentStore>) -> Self {
        Self { store }
    }

    pub async fn list(&self, filter: MemoryLayerFilter) -> QPawResult<Vec<LayeredMemoryItem>> {
        self.store.list_layered_memory(filter).await
    }

    pub async fn save_l0(&self, item: &MemoryL0) -> QPawResult<()> {
        self.store.save_l0(item).await
    }

    pub async fn save_l1_concept(&self, item: &MemoryL1Concept) -> QPawResult<()> {
        self.store.save_l1_concept(item).await
    }

    pub async fn save_l1_relation(&self, item: &MemoryL1Relation) -> QPawResult<()> {
        self.store.save_l1_relation(item).await
    }

    pub async fn save_l2(&self, item: &MemoryL2Event) -> QPawResult<()> {
        self.store.save_l2(item).await
    }

    pub async fn save_l3(&self, item: &MemoryL3Reflection) -> QPawResult<()> {
        self.store.save_l3(item).await
    }
}
