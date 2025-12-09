use common::types::{DeleteOp, InsertOp, NoopOp, OperationKind, ReplaceOp};

fn map_index_after_deletion(i: usize, del_start: usize, del_end: usize) -> usize {
    if i <= del_start {
        i
    } else if i >= del_end {
        i - (del_end - del_start)
    } else {
        del_start
    }
}

fn map_index_after_insertion(i: usize, ins_pos: usize, ins_len: usize) -> usize {
    if i < ins_pos { i } else { i + ins_len }
}

pub fn transform(op_in: OperationKind, op_prev: OperationKind) -> OperationKind {
    match op_in {
        OperationKind::Noop(_) => op_in,

        OperationKind::Insert(mut op) => match op_prev {
            OperationKind::Noop(_) => OperationKind::Insert(op),

            OperationKind::Insert(prev) => {
                // If previous insert was before us (or at same spot with lower ID), we shift right
                if prev.index < op.index
                    || (prev.index == op.index && prev.client_id < op.client_id)
                {
                    op.index += prev.text.len() as u32;
                }
                OperationKind::Insert(op)
            }

            OperationKind::Delete(prev) => {
                // Map our insertion point past the deletion
                op.index = map_index_after_deletion(
                    op.index as usize,
                    prev.start as usize,
                    prev.end as usize,
                ) as u32;
                OperationKind::Insert(op)
            }

            OperationKind::Replace(prev) => {
                // Replace is effectively Delete then Insert
                // Map past deletion
                let after_del = map_index_after_deletion(
                    op.index as usize,
                    prev.start as usize,
                    prev.end as usize,
                );
                // Map past insertion (at prev.start)
                op.index =
                    map_index_after_insertion(after_del, prev.start as usize, prev.text.len())
                        as u32;
                OperationKind::Insert(op)
            }
        },

        OperationKind::Delete(mut op) => match op_prev {
            OperationKind::Noop(_) => OperationKind::Delete(op),

            OperationKind::Insert(prev) => {
                // If insert is before our delete start, shift both start and end
                if (prev.index as u32) <= op.start {
                    op.start += prev.text.len() as u32;
                    op.end += prev.text.len() as u32;
                }
                // If insert is inside our delete range, we expand to include it (simplification)
                else if (prev.index as u32) < op.end {
                    op.end += prev.text.len() as u32;
                }
                // If insert is after, no change
                OperationKind::Delete(op)
            }

            OperationKind::Delete(prev) => {
                // Map start and end through the previous deletion
                let new_start = map_index_after_deletion(
                    op.start as usize,
                    prev.start as usize,
                    prev.end as usize,
                );
                let new_end = map_index_after_deletion(
                    op.end as usize,
                    prev.start as usize,
                    prev.end as usize,
                );

                // If range collapsed, return Noop
                if new_start == new_end {
                    OperationKind::Noop(NoopOp {
                        client_id: op.client_id,
                        client_version: op.client_version,
                    })
                } else {
                    op.start = new_start as u32;
                    op.end = new_end as u32;
                    OperationKind::Delete(op)
                }
            }

            OperationKind::Replace(prev) => {
                // Replace = Delete + Insert
                // Transform against Delete
                let start_after_del = map_index_after_deletion(
                    op.start as usize,
                    prev.start as usize,
                    prev.end as usize,
                );
                let end_after_del = map_index_after_deletion(
                    op.end as usize,
                    prev.start as usize,
                    prev.end as usize,
                );

                if start_after_del == end_after_del {
                    return OperationKind::Noop(NoopOp {
                        client_id: op.client_id,
                        client_version: op.client_version,
                    });
                }

                // Transform against Insert (at prev.start)
                let mut temp_op = DeleteOp {
                    start: start_after_del as u32,
                    end: end_after_del as u32,
                    ..op.clone()
                };

                // Logic from Delete vs Insert above
                let ins_index = prev.start;
                let ins_len = prev.text.len();

                if (ins_index as u32) <= temp_op.start {
                    temp_op.start += ins_len as u32;
                    temp_op.end += ins_len as u32;
                } else if (ins_index as u32) < temp_op.end {
                    temp_op.end += ins_len as u32;
                }

                OperationKind::Delete(temp_op)
            }
        },

        OperationKind::Replace(mut op) => match op_prev {
            OperationKind::Noop(_) => OperationKind::Replace(op),

            OperationKind::Insert(prev) => {
                // Adjust start/end like Delete
                if (prev.index as u32) <= op.start {
                    op.start += prev.text.len() as u32;
                    op.end += prev.text.len() as u32;
                } else if (prev.index as u32) < op.end {
                    op.end += prev.text.len() as u32;
                }
                OperationKind::Replace(op)
            }

            OperationKind::Delete(prev) => {
                // Adjust start/end like Delete
                let new_start = map_index_after_deletion(
                    op.start as usize,
                    prev.start as usize,
                    prev.end as usize,
                );
                let new_end = map_index_after_deletion(
                    op.end as usize,
                    prev.start as usize,
                    prev.end as usize,
                );

                if new_start == new_end {
                    // Range collapsed, but we still have text to insert!
                    // Becomes an Insert
                    OperationKind::Insert(InsertOp {
                        index: new_start as u32,
                        text: op.text,
                        client_id: op.client_id,
                        client_version: op.client_version,
                    })
                } else {
                    op.start = new_start as u32;
                    op.end = new_end as u32;
                    OperationKind::Replace(op)
                }
            }

            OperationKind::Replace(prev) => {
                // Map range against prev (Delete + Insert)
                // Map against Delete part
                let start_after_del = map_index_after_deletion(
                    op.start as usize,
                    prev.start as usize,
                    prev.end as usize,
                );
                let end_after_del = map_index_after_deletion(
                    op.end as usize,
                    prev.start as usize,
                    prev.end as usize,
                );

                // Map against Insert part
                let start_final = map_index_after_insertion(
                    start_after_del,
                    prev.start as usize,
                    prev.text.len(),
                );
                let end_final =
                    map_index_after_insertion(end_after_del, prev.start as usize, prev.text.len());

                // If range collapsed
                if start_final == end_final {
                    OperationKind::Insert(InsertOp {
                        index: start_final as u32,
                        text: op.text,
                        client_id: op.client_id,
                        client_version: op.client_version,
                    })
                } else {
                    op.start = start_final as u32;
                    op.end = end_final as u32;
                    OperationKind::Replace(op)
                }
            }
        },
    }
}
