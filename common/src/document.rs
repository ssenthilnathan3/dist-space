use uuid::Uuid;

pub struct Document {
    pub uuid: Uuid,
    pub content: String,
    pub version: u64,
}

use crate::operation::{DeleteOp, InsertOp, OperationKind, ReplaceOp};

impl Document {
    pub fn apply_op(&mut self, op: &OperationKind) -> Result<(), String> {
        match op {
            OperationKind::Insert(InsertOp { index, text, .. }) => {
                if *index as usize > self.content.len() {
                    return Err(format!(
                        "Index out of bounds: {} > {}",
                        index,
                        self.content.len()
                    ));
                }
                self.content.insert_str(*index as usize, text);
            }
            OperationKind::Delete(DeleteOp { start, end, .. }) => {
                if *end as usize > self.content.len() || start > end {
                    return Err(format!(
                        "Invalid deletion range: {}..{} (len {})",
                        start,
                        end,
                        self.content.len()
                    ));
                }
                self.content
                    .replace_range(*start as usize..*end as usize, "");
            }
            OperationKind::Replace(ReplaceOp {
                start, end, text, ..
            }) => {
                if *end as usize > self.content.len() || start > end {
                    return Err(format!(
                        "Invalid replacement range: {}..{} (len {})",
                        start,
                        end,
                        self.content.len()
                    ));
                }
                self.content
                    .replace_range(*start as usize..*end as usize, text);
            }
            OperationKind::Noop(_) => {}
        }
        self.version += 1;
        Ok(())
    }
}
