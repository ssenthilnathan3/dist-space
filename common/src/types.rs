use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use uuid::Uuid;

use crate::workspace::{OperationProto, operation_proto::Kind};

// #[derive(Clone)]
// pub struct OpId {
//     pub server_version: String,
//     pub sequence: String,
// }
//
#[derive(Clone)]
pub struct InsertOp {
    pub index: u32,
    pub text: String,
    pub client_id: String,
    pub client_version: u64,
}
#[derive(Clone)]

pub struct DeleteOp {
    pub start: u32,
    pub end: u32,
    pub client_id: String,
    pub client_version: u64,
}
#[derive(Clone)]

pub struct ReplaceOp {
    pub start: u32,
    pub end: u32,
    pub text: String,
    pub client_id: String,
    pub client_version: u64,
}
#[derive(Clone)]

pub struct NoopOp {
    pub client_id: String,
    pub client_version: u64,
}

#[derive(Clone)]
pub enum OperationKind {
    Insert(InsertOp),
    Delete(DeleteOp),
    Replace(ReplaceOp),
    Noop(NoopOp),
}

// Engine Types
#[derive(Clone)]
pub struct Operation {
    pub op_id: u64,
    pub kind: OperationKind,
    pub doc_id: String,
    pub new_content: String,
    pub client_id: Uuid,
    pub client_version: u64,
    pub server_version: u64,
}

pub struct OperationLog {
    logs: Mutex<VecDeque<Operation>>,
}

impl Operation {
    pub fn convert_operation(proto_op: OperationProto) -> Option<OperationKind> {
        match proto_op.kind {
            Some(Kind::Insert(insert_op)) => Some(OperationKind::Insert(InsertOp {
                index: insert_op.index,
                text: insert_op.text,
                client_id: insert_op.client_id,
                client_version: insert_op.client_version,
            })),
            Some(Kind::Delete(delete_op)) => Some(OperationKind::Delete(DeleteOp {
                start: delete_op.start,
                end: delete_op.end,
                client_id: delete_op.client_id,
                client_version: delete_op.client_version,
            })),
            Some(Kind::Replace(replace_op)) => Some(OperationKind::Replace(ReplaceOp {
                start: replace_op.start,
                end: replace_op.end,
                text: replace_op.text,
                client_id: replace_op.client_id,
                client_version: replace_op.client_version,
            })),
            Some(Kind::Noop(noop_op)) => Some(OperationKind::Noop(NoopOp {
                client_id: noop_op.client_id,
                client_version: noop_op.client_version,
            })),
            None => {
                // Handle the case where no operation type was set (valid for a oneof)
                None
            }
        }
    }
}

impl OperationLog {
    pub fn new() -> Self {
        Self {
            logs: Mutex::new(VecDeque::new()),
        }
    }

    pub fn append_log(&self, op: Operation) -> Result<(), String> {
        let mut logs = self
            .logs
            .lock()
            .map_err(|e| format!("Failed to lock logs: {}", e))?;
        logs.push_back(op);
        Ok(())
    }

    pub fn append_log_arc(op_log: Arc<OperationLog>, op: Operation) -> Result<(), String> {
        let mut logs = op_log
            .logs
            .lock()
            .map_err(|e| format!("Failed to lock logs: {}", e))?;
        logs.push_back(op);
        Ok(())
    }
}
