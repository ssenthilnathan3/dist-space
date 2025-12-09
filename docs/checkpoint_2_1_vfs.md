# Checkpoint 2.1: Virtual File System (VFS) Architecture

## Overview
Currently, `dist-space` synchronizes a single string of text identified by a UUID. To evolve into a "Distributed Runtime," the system must manage a **hierarchical file system** that mirrors a standard OS directory structure.

This document details the architectural changes required to implement the **Virtual File System (VFS)**.

---

## 1. Data Model Refactoring

### Current State
```rust
pub struct ServerState {
    document: Arc<Mutex<Document>>, // Single document
    // ...
}

pub struct Document {
    pub uuid: Uuid,
    pub content: String,
    pub version: u64,
}
```

### New State: The Workspace
We will introduce a `Workspace` struct that acts as the root of the VFS.

```rust
use std::collections::HashMap;
use std::path::PathBuf;

pub struct Workspace {
    pub id: Uuid,
    pub files: HashMap<String, FileEntry>, // Key is the relative path (e.g., "src/main.rs")
    pub global_version: u64, // Monotonically increasing version for the entire workspace
}

pub struct FileEntry {
    pub id: Uuid,          // Unique ID for the file (stable across renames)
    pub content: String,   // The actual text content
    pub version: u64,      // Local version for OT on this specific file
    pub is_directory: bool,
    // Future: permissions, owner, etc.
}
```

**Key Decision**:
- **Path-based addressing**: Clients usually think in paths (`src/main.rs`).
- **UUID-based identity**: Internally, we track files by UUID so that `Rename` operations don't break history.

---

## 2. Protocol Updates

The `ServerMessage` and `OperationProto` protobuf definitions need to be expanded to support file system operations.

### New Message Types
We need to introduce a `FileSystemOp` variant to the protocol.

```protobuf
message FileSystemOp {
    oneof op {
        CreateFile create = 1;
        DeleteFile delete = 2;
        MoveFile move = 3;
    }
}

message CreateFile {
    string path = 1;
    bool is_directory = 2;
    string initial_content = 3;
}

message DeleteFile {
    string path = 1;
}

message MoveFile {
    string from_path = 1;
    string to_path = 2;
}
```

### Updated OperationProto
The existing `OperationProto` (used for text editing) needs to specify *which* file is being edited.

```protobuf
message OperationProto {
    // ... existing fields ...
    string path = 11; // The file path this operation applies to
}
```

---

## 3. Implementation Strategy

### Step 1: Server-Side VFS
1.  Create `server/src/vfs.rs`.
2.  Implement the `Workspace` struct.
3.  Implement methods: `create_file`, `get_file`, `delete_file`.
4.  Replace the single `Document` in `ServerState` with `Workspace`.

### Step 2: Protocol Expansion
1.  Update `common/proto/workspace.proto`.
2.  Recompile protobufs.
3.  Update `common/src/protocol.rs` to handle new message types.

### Step 3: Client Adaptation
1.  Update the CLI client to support a "file mode."
2.  Commands:
    - `edit <filename>`: Switches context to that file.
    - `ls`: Lists files in the workspace.
    - `touch <filename>`: Creates a file.

---

## 4. OT Implications
Operational Transformation (OT) will continue to work **per-file**.
- When an operation arrives for `src/main.rs`, the server looks up the `FileEntry` for that path.
- It retrieves the `op_log` specific to that file (or filters the global log).
- It performs transformation logic exactly as it does now.

**Concurrency Edge Case**:
- **File Deletion vs. Editing**: If User A deletes `file.txt` while User B is typing in it:
    - The `DeleteFile` operation takes precedence.
    - User B's edits are rejected (or applied to a "ghost" file depending on policy).
    - **Policy**: Strict consistency. If a file is deleted, subsequent edits to it fail with `FileNotFound`.
