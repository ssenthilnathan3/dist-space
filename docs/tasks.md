# Implementation Roadmap: Dist-Space Runtime

This document outlines the checkpoints, tasks, and deliverables required to transform the current prototype into the production-grade **Dist-Space Runtime**.

---

## Phase 1: Foundation & Stability (The "Crash Proofing")
**Goal**: Ensure the system is mathematically correct, crash-resistant, and handles basic concurrency without data corruption.

### Checkpoint 1.1: Core Algorithm Fixes 
- [x] **Fix OT Implementation (`transform.rs`)**
    - [x] Implement `Delete-Delete` transformation.
    - [x] Implement `Delete-Replace` transformation.
    - [x] Implement `Replace-Replace` transformation.
    - [x] Fix logic error in `Insert-Insert` transformation (variable shadowing bug).
- [ ] **Unit Testing Suite**
    - [ ] Add comprehensive unit tests for all OT permutations.
    - [ ] Add property-based testing (fuzzing) to ensure convergence.

### Checkpoint 1.2: Protocol Hardening
- [x] **Fix Client Encoding**
    - [x] Update client to send correct Type ID prefix in Protobuf messages.
- [x] **Error Handling**
    - [x] Remove `unwrap()` and `expect()` from server request path.
    - [x] Implement proper `Result` propagation and error logging.
- [ ] **Connection Management**
    - [ ] Implement client timeouts (heartbeats).
    - [ ] Add connection limits to prevent DoS.

---

## Phase 2: Runtime Architecture (The "OS" Features)
**Goal**: Move from single-string syncing to a full multi-file system with persistence.

### Checkpoint 2.1: Virtual File System (VFS)
- [ ] **Refactor Data Model**
    - [ ] Change `Document` to `Workspace`.
    - [ ] Implement `HashMap<Path, Document>` structure.
- [ ] **File Operations Protocol**
    - [ ] Add `CreateFile`, `DeleteFile`, `RenameFile` protocol messages.
    - [ ] Implement directory structure support.

### Checkpoint 2.2: Persistence & History
- [ ] **Operation Log Storage**
    - [ ] Integrate an embedded DB (e.g., `sled` or `rocksdb`) for the OpLog.
    - [ ] Implement log loading on server startup.
- [ ] **Checkpointing**
    - [ ] Implement "Snapshot" system (save full state every N operations).
    - [ ] Implement log truncation (keep only recent ops in memory).

---

## Phase 3: Compute & Extensibility (The "AI" Layer)
**Goal**: Enable safe, embedded execution of AI agents and plugins.

### Checkpoint 3.1: WASM Runtime Integration
- [ ] **Embed Wasmtime**
    - [ ] Integrate `wasmtime` crate into the server.
    - [ ] Define the "Host API" (functions exposed to WASM modules).
- [ ] **Plugin System**
    - [ ] Create a mechanism to load `.wasm` files as "Agents".
    - [ ] Implement the `Agent` interface (can read state, propose edits).

### Checkpoint 3.2: Permissions & Sandboxing
- [ ] **Capability System**
    - [ ] Implement permission scopes (e.g., `READ_ONLY`, `EDIT_FILE:src/*`).
    - [ ] Enforce permissions at the API boundary.

---

## Phase 4: Developer Experience (The "Interface")
**Goal**: Make it easy for developers to build on top of Dist-Space.

### Checkpoint 4.1: SDK & Client Libraries
- [ ] **Rust SDK**
    - [ ] Abstract raw Protobuf/TCP into a high-level `Client` struct.
- [ ] **TypeScript/JS SDK**
    - [ ] Build a WASM-based client for browser usage.
    - [ ] Implement WebSocket transport adapter.

### Checkpoint 4.2: CLI Tools
- [ ] **Admin CLI**
    - [ ] Tools to inspect server state, view logs, and manage connected clients.
- [ ] **Headless Client**
    - [ ] A reference implementation that syncs a local folder to the remote workspace.
