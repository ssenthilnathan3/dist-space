# Checkpoint 2.2: Persistence & History Architecture

## Overview
Currently, `dist-space` stores all state in RAM. If the server restarts, all code and history are lost. Additionally, the `OperationLog` grows unbounded, eventually causing an Out-Of-Memory (OOM) crash.

This document details the architecture for **Persistence** (saving state to disk) and **History Management** (handling the log efficiently).

---

## 1. Storage Engine Selection

We need an embedded key-value store that is fast, reliable, and Rust-native.

**Choice: `sled`**
- **Pros**: Pure Rust, embedded (no external DB process), lock-free, supports transactions.
- **Cons**: Still in beta (though stable enough for this stage).

*Alternative: `rocksdb` (via rust-rocksdb) if raw performance becomes a bottleneck.*

---

## 2. Schema Design

We will map our VFS and Operation Log to a Key-Value structure.

### Tree: `files`
Stores the current snapshot of file metadata.
- **Key**: `path` (String) -> `src/main.rs`
- **Value**: `FileMetadata` (Protobuf/Bincode) -> `{ id: uuid, version: 105, created_at: timestamp }`

### Tree: `content`
Stores the actual file content. Separated from metadata to allow listing files without loading gigabytes of text.
- **Key**: `file_id` (Uuid)
- **Value**: `String` (The full file content)

### Tree: `ops`
The append-only log of all operations.
- **Key**: `global_seq_num` (u64, BigEndian)
- **Value**: `Operation` (Protobuf)

### Tree: `snapshots`
Periodic snapshots of file content to speed up recovery.
- **Key**: `file_id:version`
- **Value**: `String` (Content at that version)

---

## 3. Implementation Strategy

### Step 1: The Storage Layer
1.  Add `sled` dependency to `server/Cargo.toml`.
2.  Create `server/src/storage.rs`.
3.  Implement a `Storage` struct that wraps the `sled::Db`.

```rust
pub struct Storage {
    db: sled::Db,
    ops_tree: sled::Tree,
    files_tree: sled::Tree,
}

impl Storage {
    pub fn append_op(&self, op: &Operation) -> Result<()>;
    pub fn get_file(&self, path: &str) -> Result<Option<FileEntry>>;
    pub fn save_file(&self, path: &str, entry: &FileEntry);
}
```

### Step 2: Server Startup & Recovery
1.  On startup, `ServerState` initializes `Storage`.
2.  It loads the latest state of the `Workspace` from `files_tree`.
3.  It caches the active file contents in memory (RAM is the cache).

### Step 3: Log Truncation (Garbage Collection)
We cannot keep every keystroke forever in RAM.

**Strategy**:
1.  **Hot Log (RAM)**: Keep the last 1,000 operations in `VecDeque` for fast OT.
2.  **Cold Log (Disk)**: All operations are persisted to `sled`.
3.  **Hydration**: If a client connects with a very old version (e.g., version 50 when server is at 5000), the server fetches missing ops from `sled` instead of RAM.

---

## 4. Time Travel & Replay

With the persisted `ops` tree, we can implement "Time Travel."

### API
```rust
pub fn checkout_version(&self, target_version: u64) -> Workspace {
    // 1. Find the nearest snapshot before target_version
    // 2. Replay ops from snapshot up to target_version
    // 3. Return the reconstructed state
}
```

This allows the frontend to have a "History Slider" UI, scrubbing back through the evolution of the codebase.
