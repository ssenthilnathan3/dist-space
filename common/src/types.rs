use std::sync::{Arc, Mutex};

// Engine Types
pub struct OperationLog {
    logs: Mutex<Vec<Operation>>,
}

impl OperationLog {
    pub fn new() -> Self {
        Self {
            logs: Mutex::new(Vec::new()),
        }
    }

    pub fn append_log(&self, op: Operation) -> Result<(), String> {
        let mut logs = self
            .logs
            .lock()
            .map_err(|e| format!("Failed to lock logs: {}", e))?;
        logs.push(op);
        Ok(())
    }

    pub fn append_log_arc(op_log: Arc<OperationLog>, op: Operation) -> Result<(), String> {
        let mut logs = op_log
            .logs
            .lock()
            .map_err(|e| format!("Failed to lock logs: {}", e))?;
        logs.push(op);
        Ok(())
    }
}

#[derive(Clone)]
pub struct Operation {
    pub doc_id: String,
    pub new_content: String,
    pub client_version: u32,
}
