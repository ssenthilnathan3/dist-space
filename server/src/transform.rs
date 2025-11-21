use common::types::{DeleteOp, InsertOp, NoopOp, OperationKind, ReplaceOp};

fn clamp(i: usize, lo: usize, hi: usize) -> usize {
    return hi.min(i).max(lo);
}

fn overlap(a0: usize, b0: usize, a1: usize, b1: usize) -> usize {
    let inner = b1.min(a1) - b0.max(a0);
    return inner.max(0);
}

fn map_index_after_deletion(i: usize, del_start: usize, del_end: usize) -> usize {
    if i <= del_start {
        return i;
    } else if i >= del_end {
        return i - (del_end - del_start);
    } else {
        del_start
    }
}

fn map_range_after_deletion(
    range_start: usize,
    range_end: usize,
    del_start: usize,
    del_end: usize,
) -> (usize, usize) {
    // Map each index independently
    let mapped_start = map_index_after_deletion(range_start, del_start, del_end);
    let mapped_end = map_index_after_deletion(range_end, del_start, del_end);

    // Ensure valid range (start ≤ end)
    if mapped_start > mapped_end {
        (mapped_end, mapped_end) // Empty range
    } else {
        (mapped_start, mapped_end)
    }
}

pub fn transform(op_in: OperationKind, op_prev: OperationKind) -> OperationKind {
    match op_in {
        OperationKind::Noop(NoopOp {
            client_id,
            client_version,
        }) => {
            return OperationKind::Noop(NoopOp {
                client_id,
                client_version,
            });
        }

        OperationKind::Insert(InsertOp {
            index: in_index,
            text,
            client_id: in_client_id,
            client_version: in_client_version,
        }) => match op_prev {
            OperationKind::Insert(InsertOp {
                index: prev_index,
                text,
                client_id,
                client_version,
            }) => {
                if prev_index < in_index {
                    OperationKind::Insert(InsertOp {
                        index: prev_index + 1,
                        text,
                        client_id,
                        client_version,
                    })
                } else {
                    OperationKind::Insert(InsertOp {
                        index: prev_index,
                        text,
                        client_id,
                        client_version,
                    })
                }
            }
            OperationKind::Delete(DeleteOp {
                start: del_start,
                end: del_end,
                client_id,
                client_version,
            }) => {
                let new_count = map_index_after_deletion(
                    in_index as usize,
                    del_start as usize,
                    del_end as usize,
                );

                OperationKind::Insert(InsertOp {
                    index: new_count as u32,
                    text,
                    client_id,
                    client_version,
                })
            }
            OperationKind::Replace(ReplaceOp {
                start,
                end,
                text,
                client_id,
                client_version,
            }) => {}
            OperationKind::Noop(NoopOp {
                client_id,
                client_version,
            }) => return op_in,
        },

        OperationKind::Delete(DeleteOp {
            start,
            end,
            client_id,
            client_version,
        }) => match op_prev {
            OperationKind::Insert(InsertOp {
                index,
                text,
                client_id,
                client_version,
            }) => {}
            OperationKind::Delete(DeleteOp {
                start,
                end,
                client_id,
                client_version,
            }) => {}
            OperationKind::Replace(ReplaceOp {
                start,
                end,
                text,
                client_id,
                client_version,
            }) => {}
            OperationKind::Noop(NoopOp {
                client_id,
                client_version,
            }) => return op_in,
        },

        OperationKind::Replace(ReplaceOp {
            start,
            end,
            text,
            client_id,
            client_version,
        }) => {}
    }
}
