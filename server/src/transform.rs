use common::operation::{DeleteOp, InsertOp, NoopOp, OperationKind};

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

#[cfg(test)]
mod tests {
    use super::*;
    use common::operation::ReplaceOp;

    // ============================================
    // HELPER FUNCTIONS FOR TESTING
    // ============================================

    fn make_insert(index: u32, text: &str, client_id: &str, version: u64) -> OperationKind {
        OperationKind::Insert(InsertOp {
            index,
            text: text.to_string(),
            client_id: client_id.to_string(),
            client_version: version,
        })
    }

    fn make_delete(start: u32, end: u32, client_id: &str, version: u64) -> OperationKind {
        OperationKind::Delete(DeleteOp {
            start,
            end,
            client_id: client_id.to_string(),
            client_version: version,
        })
    }

    fn make_replace(start: u32, end: u32, text: &str, client_id: &str, version: u64) -> OperationKind {
        OperationKind::Replace(ReplaceOp {
            start,
            end,
            text: text.to_string(),
            client_id: client_id.to_string(),
            client_version: version,
        })
    }

    fn make_noop(client_id: &str, version: u64) -> OperationKind {
        OperationKind::Noop(NoopOp {
            client_id: client_id.to_string(),
            client_version: version,
        })
    }

    /// Apply an operation to a string document
    fn apply_op(doc: &mut String, op: &OperationKind) -> Result<(), String> {
        match op {
            OperationKind::Insert(InsertOp { index, text, .. }) => {
                if *index as usize > doc.len() {
                    return Err(format!("Insert index {} out of bounds (len {})", index, doc.len()));
                }
                doc.insert_str(*index as usize, text);
                Ok(())
            }
            OperationKind::Delete(DeleteOp { start, end, .. }) => {
                if *end as usize > doc.len() || start > end {
                    return Err(format!("Invalid delete range {}..{} (len {})", start, end, doc.len()));
                }
                doc.replace_range(*start as usize..*end as usize, "");
                Ok(())
            }
            OperationKind::Replace(ReplaceOp { start, end, text, .. }) => {
                if *end as usize > doc.len() || start > end {
                    return Err(format!("Invalid replace range {}..{} (len {})", start, end, doc.len()));
                }
                doc.replace_range(*start as usize..*end as usize, text);
                Ok(())
            }
            OperationKind::Noop(_) => Ok(()),
        }
    }

    // ============================================
    // UNIT TESTS: Noop Transformations
    // ============================================

    #[test]
    fn test_noop_vs_noop() {
        let op = make_noop("A", 1);
        let prev = make_noop("B", 1);
        let result = transform(op.clone(), prev);
        assert!(matches!(result, OperationKind::Noop(_)));
    }

    #[test]
    fn test_noop_vs_insert() {
        let op = make_noop("A", 1);
        let prev = make_insert(0, "hello", "B", 1);
        let result = transform(op.clone(), prev);
        assert!(matches!(result, OperationKind::Noop(_)));
    }

    #[test]
    fn test_noop_vs_delete() {
        let op = make_noop("A", 1);
        let prev = make_delete(0, 5, "B", 1);
        let result = transform(op.clone(), prev);
        assert!(matches!(result, OperationKind::Noop(_)));
    }

    // ============================================
    // UNIT TESTS: Insert vs Insert
    // ============================================

    #[test]
    fn test_insert_insert_before() {
        // Insert "X" at 5, previous insert "YY" at 2
        // Result: Insert "X" at 7 (shifted by 2)
        let op = make_insert(5, "X", "A", 1);
        let prev = make_insert(2, "YY", "B", 1);
        let result = transform(op, prev);
        
        if let OperationKind::Insert(insert) = result {
            assert_eq!(insert.index, 7);
            assert_eq!(insert.text, "X");
        } else {
            panic!("Expected Insert, got {:?}", result);
        }
    }

    #[test]
    fn test_insert_insert_after() {
        // Insert "X" at 2, previous insert "YY" at 5
        // Result: Insert "X" at 2 (no shift)
        let op = make_insert(2, "X", "A", 1);
        let prev = make_insert(5, "YY", "B", 1);
        let result = transform(op, prev);
        
        if let OperationKind::Insert(insert) = result {
            assert_eq!(insert.index, 2);
        } else {
            panic!("Expected Insert");
        }
    }

    #[test]
    fn test_insert_insert_same_position_tie_break() {
        // Both insert at position 3, client_id determines winner
        // "A" < "B" means A wins (stays in place), B shifts
        let op = make_insert(3, "X", "B", 1);  // B's operation
        let prev = make_insert(3, "YY", "A", 1);  // A's already applied
        let result = transform(op, prev);
        
        if let OperationKind::Insert(insert) = result {
            // B should shift because A's client_id < B's client_id
            assert_eq!(insert.index, 5);  // 3 + len("YY")
        } else {
            panic!("Expected Insert");
        }
    }

    #[test]
    fn test_insert_insert_same_position_tie_break_reverse() {
        // "A" < "B", so A doesn't shift when B is previous
        let op = make_insert(3, "X", "A", 1);  // A's operation
        let prev = make_insert(3, "YY", "B", 1);  // B's already applied
        let result = transform(op, prev);
        
        if let OperationKind::Insert(insert) = result {
            // A should not shift because A's client_id < B's client_id
            assert_eq!(insert.index, 3);
        } else {
            panic!("Expected Insert");
        }
    }

    // ============================================
    // UNIT TESTS: Insert vs Delete
    // ============================================

    #[test]
    fn test_insert_delete_before_range() {
        // Insert at 2, delete 5..8
        // Result: Insert at 2 (no change)
        let op = make_insert(2, "X", "A", 1);
        let prev = make_delete(5, 8, "B", 1);
        let result = transform(op, prev);
        
        if let OperationKind::Insert(insert) = result {
            assert_eq!(insert.index, 2);
        } else {
            panic!("Expected Insert");
        }
    }

    #[test]
    fn test_insert_delete_after_range() {
        // Insert at 10, delete 5..8 (removes 3 chars)
        // Result: Insert at 7 (10 - 3)
        let op = make_insert(10, "X", "A", 1);
        let prev = make_delete(5, 8, "B", 1);
        let result = transform(op, prev);
        
        if let OperationKind::Insert(insert) = result {
            assert_eq!(insert.index, 7);
        } else {
            panic!("Expected Insert");
        }
    }

    #[test]
    fn test_insert_delete_inside_range() {
        // Insert at 6, delete 5..8
        // Result: Insert at 5 (moved to deletion start)
        let op = make_insert(6, "X", "A", 1);
        let prev = make_delete(5, 8, "B", 1);
        let result = transform(op, prev);
        
        if let OperationKind::Insert(insert) = result {
            assert_eq!(insert.index, 5);  // Mapped to deletion start
        } else {
            panic!("Expected Insert");
        }
    }

    // ============================================
    // UNIT TESTS: Delete vs Insert
    // ============================================

    #[test]
    fn test_delete_insert_before() {
        // Delete 5..8, insert "XX" at 2
        // Result: Delete 7..10 (shifted by 2)
        let op = make_delete(5, 8, "A", 1);
        let prev = make_insert(2, "XX", "B", 1);
        let result = transform(op, prev);
        
        if let OperationKind::Delete(delete) = result {
            assert_eq!(delete.start, 7);
            assert_eq!(delete.end, 10);
        } else {
            panic!("Expected Delete");
        }
    }

    #[test]
    fn test_delete_insert_after() {
        // Delete 2..4, insert "XX" at 10
        // Result: Delete 2..4 (no change)
        let op = make_delete(2, 4, "A", 1);
        let prev = make_insert(10, "XX", "B", 1);
        let result = transform(op, prev);
        
        if let OperationKind::Delete(delete) = result {
            assert_eq!(delete.start, 2);
            assert_eq!(delete.end, 4);
        } else {
            panic!("Expected Delete");
        }
    }

    #[test]
    fn test_delete_insert_inside_range() {
        // Delete 2..8, insert "XXX" at 5
        // Result: Delete 2..11 (expanded by 3)
        let op = make_delete(2, 8, "A", 1);
        let prev = make_insert(5, "XXX", "B", 1);
        let result = transform(op, prev);
        
        if let OperationKind::Delete(delete) = result {
            assert_eq!(delete.start, 2);
            assert_eq!(delete.end, 11);  // 8 + 3
        } else {
            panic!("Expected Delete");
        }
    }

    // ============================================
    // UNIT TESTS: Delete vs Delete
    // ============================================

    #[test]
    fn test_delete_delete_non_overlapping_before() {
        // Delete 10..15, previous delete 2..5
        // Result: Delete 7..12 (shifted by 3)
        let op = make_delete(10, 15, "A", 1);
        let prev = make_delete(2, 5, "B", 1);
        let result = transform(op, prev);
        
        if let OperationKind::Delete(delete) = result {
            assert_eq!(delete.start, 7);
            assert_eq!(delete.end, 12);
        } else {
            panic!("Expected Delete");
        }
    }

    #[test]
    fn test_delete_delete_non_overlapping_after() {
        // Delete 2..5, previous delete 10..15
        // Result: Delete 2..5 (no change)
        let op = make_delete(2, 5, "A", 1);
        let prev = make_delete(10, 15, "B", 1);
        let result = transform(op, prev);
        
        if let OperationKind::Delete(delete) = result {
            assert_eq!(delete.start, 2);
            assert_eq!(delete.end, 5);
        } else {
            panic!("Expected Delete");
        }
    }

    #[test]
    fn test_delete_delete_fully_covered() {
        // Delete 5..8, previous delete 2..12 (fully covers our delete)
        // Result: Noop (our delete was already deleted)
        let op = make_delete(5, 8, "A", 1);
        let prev = make_delete(2, 12, "B", 1);
        let result = transform(op, prev);
        
        assert!(matches!(result, OperationKind::Noop(_)), "Expected Noop when delete is fully covered");
    }

    #[test]
    fn test_delete_delete_partial_overlap_left() {
        // Delete 5..10, previous delete 3..7
        // After prev: original indices [7,10) become [3,6)
        let op = make_delete(5, 10, "A", 1);
        let prev = make_delete(3, 7, "B", 1);
        let result = transform(op, prev);
        
        if let OperationKind::Delete(delete) = result {
            // 5 maps to 3 (clamped to del_start), 10 maps to 6 (10 - 4)
            assert_eq!(delete.start, 3);
            assert_eq!(delete.end, 6);
        } else {
            panic!("Expected Delete, got {:?}", result);
        }
    }

    // ============================================
    // UNIT TESTS: Insert vs Replace
    // ============================================

    #[test]
    fn test_insert_replace_before() {
        // Insert at 2, replace 5..8 with "XYZ"
        // Result: Insert at 2 (no change)
        let op = make_insert(2, "A", "A", 1);
        let prev = make_replace(5, 8, "XYZ", "B", 1);
        let result = transform(op, prev);
        
        if let OperationKind::Insert(insert) = result {
            assert_eq!(insert.index, 2);
        } else {
            panic!("Expected Insert");
        }
    }

    #[test]
    fn test_insert_replace_after() {
        // Insert at 12, replace 5..8 with "X" (removes 3, adds 1)
        // Result: Insert at 10 (12 - 3 + 1)
        let op = make_insert(12, "A", "A", 1);
        let prev = make_replace(5, 8, "X", "B", 1);
        let result = transform(op, prev);
        
        if let OperationKind::Insert(insert) = result {
            // After delete: 12 -> 9 (12 - 3)
            // After insert at 5: 9 -> 10 (9 + 1)
            assert_eq!(insert.index, 10);
        } else {
            panic!("Expected Insert");
        }
    }

    // ============================================
    // UNIT TESTS: Delete vs Replace
    // ============================================

    #[test]
    fn test_delete_replace_non_overlapping() {
        // Delete 10..15, replace 2..5 with "XX"
        // Result: Delete adjusted
        let op = make_delete(10, 15, "A", 1);
        let prev = make_replace(2, 5, "XX", "B", 1);
        let result = transform(op, prev);
        
        if let OperationKind::Delete(delete) = result {
            // After delete of 2..5: 10 -> 7, 15 -> 12
            // After insert of "XX" at 2: 7 -> 9, 12 -> 14
            assert_eq!(delete.start, 9);
            assert_eq!(delete.end, 14);
        } else {
            panic!("Expected Delete, got {:?}", result);
        }
    }

    // ============================================
    // UNIT TESTS: Replace vs Replace
    // ============================================

    #[test]
    fn test_replace_replace_non_overlapping() {
        // Replace 10..15 with "AA", previous replace 2..5 with "BB"
        let op = make_replace(10, 15, "AA", "A", 1);
        let prev = make_replace(2, 5, "BB", "B", 1);
        let result = transform(op, prev);
        
        if let OperationKind::Replace(replace) = result {
            // After delete 2..5: 10 -> 7, 15 -> 12
            // After insert "BB" at 2: 7 -> 9, 12 -> 14
            assert_eq!(replace.start, 9);
            assert_eq!(replace.end, 14);
            assert_eq!(replace.text, "AA");
        } else {
            panic!("Expected Replace, got {:?}", result);
        }
    }

    #[test]
    fn test_replace_replace_fully_covered() {
        // Replace 5..8 with "X", previous replace 2..12 with "Y"
        // Result: Insert (range collapsed)
        let op = make_replace(5, 8, "X", "A", 1);
        let prev = make_replace(2, 12, "Y", "B", 1);
        let result = transform(op, prev);
        
        // After prev's delete: both 5 and 8 map to 2
        // After prev's insert: both map to 3
        // Since range collapsed, becomes Insert
        assert!(matches!(result, OperationKind::Insert(_)), "Expected Insert when replace range is fully covered");
    }

    // ============================================
    // UNIT TESTS: Replace vs Delete (special cases)
    // ============================================

    #[test]
    fn test_replace_delete_range_collapsed() {
        // Replace 5..8 with "NEW", previous delete 3..10
        // Range 5..8 is fully within 3..10, so it collapses
        // Result: Insert at mapped position
        let op = make_replace(5, 8, "NEW", "A", 1);
        let prev = make_delete(3, 10, "B", 1);
        let result = transform(op, prev);
        
        if let OperationKind::Insert(insert) = result {
            assert_eq!(insert.index, 3);  // Both 5 and 8 map to 3
            assert_eq!(insert.text, "NEW");
        } else {
            panic!("Expected Insert when replace range collapses, got {:?}", result);
        }
    }

    // ============================================
    // CONVERGENCE TESTS
    // ============================================

    /// Helper to test convergence: applying ops in different orders should
    /// produce the same result after transformation
    fn test_convergence(initial: &str, op_a: OperationKind, op_b: OperationKind) {
        // Path 1: Apply A, then transform and apply B
        let mut doc1 = initial.to_string();
        apply_op(&mut doc1, &op_a).expect("Apply A failed");
        let transformed_b = transform(op_b.clone(), op_a.clone());
        apply_op(&mut doc1, &transformed_b).expect("Apply transformed B failed");

        // Path 2: Apply B, then transform and apply A
        let mut doc2 = initial.to_string();
        apply_op(&mut doc2, &op_b).expect("Apply B failed");
        let transformed_a = transform(op_a.clone(), op_b.clone());
        apply_op(&mut doc2, &transformed_a).expect("Apply transformed A failed");

        assert_eq!(doc1, doc2, 
            "Convergence failed!\nInitial: {:?}\nOp A: {:?}\nOp B: {:?}\nPath 1 result: {:?}\nPath 2 result: {:?}",
            initial, op_a, op_b, doc1, doc2);
    }

    #[test]
    fn test_convergence_insert_insert() {
        test_convergence(
            "hello world",
            make_insert(5, "X", "A", 1),
            make_insert(8, "Y", "B", 1),
        );
    }

    #[test]
    fn test_convergence_insert_insert_same_position() {
        test_convergence(
            "hello world",
            make_insert(5, "X", "A", 1),
            make_insert(5, "Y", "B", 1),
        );
    }

    #[test]
    fn test_convergence_insert_delete() {
        test_convergence(
            "hello world",
            make_insert(2, "XX", "A", 1),
            make_delete(3, 7, "B", 1),
        );
    }

    #[test]
    fn test_convergence_delete_delete_overlapping() {
        test_convergence(
            "hello world",
            make_delete(2, 7, "A", 1),
            make_delete(5, 9, "B", 1),
        );
    }
}

// ============================================
// PROPERTY-BASED TESTS (FUZZING) 
// ============================================

#[cfg(test)]
#[allow(dead_code)]
mod proptests {
    use super::*;
    use common::operation::ReplaceOp;
    use proptest::prelude::*;

    // Maximum document size for testing
    const MAX_DOC_SIZE: usize = 100;

    /// Generate a random Insert operation valid for a document of given length
    fn arb_insert(doc_len: usize) -> impl Strategy<Value = OperationKind> {
        let max_idx = doc_len;  // can insert at end
        (0..=max_idx as u32, "[a-z]{1,5}", "[A-Z]", 0u64..100)
            .prop_map(|(index, text, client_id, version)| {
                OperationKind::Insert(InsertOp {
                    index,
                    text,
                    client_id,
                    client_version: version,
                })
            })
    }

    /// Generate a random Delete operation valid for a document of given length
    fn arb_delete(doc_len: usize) -> impl Strategy<Value = OperationKind> {
        if doc_len == 0 {
            // Can't delete from empty doc, return Noop
            return Just(OperationKind::Noop(NoopOp {
                client_id: "X".to_string(),
                client_version: 0,
            })).boxed();
        }
        
        (0..doc_len as u32, "[A-Z]", 0u64..100)
            .prop_flat_map(move |(start, client_id, version)| {
                let end_max = doc_len as u32;
                (Just(start), (start + 1)..=end_max, Just(client_id), Just(version))
            })
            .prop_map(|(start, end, client_id, version)| {
                OperationKind::Delete(DeleteOp {
                    start,
                    end,
                    client_id,
                    client_version: version,
                })
            })
            .boxed()
    }

    /// Generate a random Replace operation valid for a document of given length
    fn arb_replace(doc_len: usize) -> impl Strategy<Value = OperationKind> {
        if doc_len == 0 {
            // Can't replace from empty doc, return Insert instead
            return arb_insert(0).boxed();
        }
        
        (0..doc_len as u32, "[a-z]{1,3}", "[A-Z]", 0u64..100)
            .prop_flat_map(move |(start, text, client_id, version)| {
                let end_max = doc_len as u32;
                (Just(start), (start + 1)..=end_max, Just(text), Just(client_id), Just(version))
            })
            .prop_map(|(start, end, text, client_id, version)| {
                OperationKind::Replace(ReplaceOp {
                    start,
                    end,
                    text,
                    client_id,
                    client_version: version,
                })
            })
            .boxed()
    }

    /// Generate any random operation valid for a document of given length
    fn arb_operation(doc_len: usize) -> impl Strategy<Value = OperationKind> {
        prop_oneof![
            arb_insert(doc_len),
            arb_delete(doc_len),
            arb_replace(doc_len),
        ]
    }

    /// Apply an operation to a document, returning error if invalid
    fn apply_op(doc: &mut String, op: &OperationKind) -> Result<(), String> {
        match op {
            OperationKind::Insert(InsertOp { index, text, .. }) => {
                if *index as usize > doc.len() {
                    return Err(format!("Insert index {} out of bounds", index));
                }
                doc.insert_str(*index as usize, text);
                Ok(())
            }
            OperationKind::Delete(DeleteOp { start, end, .. }) => {
                if *end as usize > doc.len() || start > end {
                    return Err(format!("Invalid delete range {}..{}", start, end));
                }
                doc.replace_range(*start as usize..*end as usize, "");
                Ok(())
            }
            OperationKind::Replace(ReplaceOp { start, end, text, .. }) => {
                if *end as usize > doc.len() || start > end {
                    return Err(format!("Invalid replace range {}..{}", start, end));
                }
                doc.replace_range(*start as usize..*end as usize, text);
                Ok(())
            }
            OperationKind::Noop(_) => Ok(()),
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000))]

        /// Property: OT convergence - applying ops in different orders with transformation
        /// should produce the same final document state
        #[test]
        fn prop_convergence_insert_insert(
            initial in "[a-z]{5,20}",
        ) {
            let doc_len = initial.len();
            
            // Generate two insert operations
            let idx1 = (doc_len / 3) as u32;
            let idx2 = (doc_len * 2 / 3) as u32;
            
            let op_a = OperationKind::Insert(InsertOp {
                index: idx1,
                text: "AAA".to_string(),
                client_id: "A".to_string(),
                client_version: 1,
            });
            
            let op_b = OperationKind::Insert(InsertOp {
                index: idx2,
                text: "BBB".to_string(),
                client_id: "B".to_string(),
                client_version: 1,
            });

            // Path 1: Apply A, then transform and apply B
            let mut doc1 = initial.clone();
            apply_op(&mut doc1, &op_a).unwrap();
            let transformed_b = transform(op_b.clone(), op_a.clone());
            apply_op(&mut doc1, &transformed_b).unwrap();

            // Path 2: Apply B, then transform and apply A
            let mut doc2 = initial.clone();
            apply_op(&mut doc2, &op_b).unwrap();
            let transformed_a = transform(op_a.clone(), op_b.clone());
            apply_op(&mut doc2, &transformed_a).unwrap();

            prop_assert_eq!(doc1, doc2, "Convergence failed for insert-insert");
        }

        /// Property: Transforming an operation against Noop should preserve it
        #[test]
        fn prop_noop_identity(
            initial in "[a-z]{5,20}",
        ) {
            let doc_len = initial.len();
            let idx = (doc_len / 2) as u32;
            
            let op = OperationKind::Insert(InsertOp {
                index: idx,
                text: "X".to_string(),
                client_id: "A".to_string(),
                client_version: 1,
            });
            
            let noop = OperationKind::Noop(NoopOp {
                client_id: "B".to_string(),
                client_version: 1,
            });

            let result = transform(op.clone(), noop);
            
            // Should be equivalent operation
            if let (OperationKind::Insert(orig), OperationKind::Insert(res)) = (&op, &result) {
                prop_assert_eq!(orig.index, res.index);
                prop_assert_eq!(&orig.text, &res.text);
            } else {
                panic!("Expected Insert");
            }
        }

        /// Property: Delete of already-deleted range becomes Noop
        #[test]
        fn prop_delete_idempotence(
            initial in "[a-z]{10,30}",
        ) {
            let doc_len = initial.len();
            let start = (doc_len / 4) as u32;
            let end = (doc_len * 3 / 4) as u32;
            
            // Two clients try to delete the same range
            let op_a = OperationKind::Delete(DeleteOp {
                start,
                end,
                client_id: "A".to_string(),
                client_version: 1,
            });
            
            let op_b = OperationKind::Delete(DeleteOp {
                start,
                end,
                client_id: "B".to_string(),
                client_version: 1,
            });

            // After A is applied, B should become Noop when transformed
            let transformed_b = transform(op_b, op_a);
            prop_assert!(matches!(transformed_b, OperationKind::Noop(_)),
                "Delete of already-deleted range should become Noop");
        }
    }
}
