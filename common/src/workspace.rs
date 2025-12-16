use std::collections::HashMap;
use uuid::Uuid;

use crate::Document;

pub struct Workspace {
    pub id: Uuid,

    /// Key is the relative path (e.g., "src/main.rs")
    pub files: HashMap<String, Document>,

    /// Monotonically increasing version for the entire workspace
    pub global_version: u64,
}
