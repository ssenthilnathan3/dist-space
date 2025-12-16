**An experiental distributed, deterministic workspace engine that synchronizes code edits across humans and AI agents, maintains a bounded operation log, resolves concurrent edits, and exposes real-time snapshots and context for tools.**

## Vision

Dist-Space is a distributed runtime designed to be the backbone of the next generation of software development. It transforms the codebase from a static set of files on a disk into a **live, collaborative entity** where humans and AI agents interact in real-time with zero latency.

- Everyone connects to it.  
- Everyone sees the same workspace.  
- Everyone's actions funnel through it.  
- AI agents have the same view as humans.  
- The system resolves conflicts + maintains causality + logs history.

## Current Features

### Core OT Engine
- **Operational Transformation**: Full implementation of Insert, Delete, Replace, and Noop operations
- **Conflict Resolution**: Deterministic tie-breaking for concurrent edits
- **Convergence Guarantee**: All clients converge to the same state regardless of operation order

### Protocol & Communication
- **Length-prefixed binary protocol** with type IDs for efficient message framing
- **Protobuf serialization** for operations and sync messages
- **Ping/Pong heartbeats** for client liveness detection

### Connection Management
- **Client timeouts**: Automatic disconnection of unresponsive clients (30s timeout)
- **Connection limits**: DoS protection with max 100 concurrent clients
- **Graceful cleanup**: Proper resource cleanup on client disconnect

### Testing
- **30 unit tests** covering all OT permutations
- **Property-based testing (fuzzing)** with proptest for convergence verification

## Quick Start

### Start the Server
```bash
cargo run -p server
```

### Run a Client
```bash
cargo run -p client
```

Or the test client:
```bash
cargo run -p test_client
```

### Run Tests
```bash
# Run OT unit tests
cargo test -p server

# Run integration tests
cargo run -p tests
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Dist-Space Server                     │
├─────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
│  │   Reader    │  │   Writer    │  │   Broadcaster   │  │
│  │  (per-conn) │  │  (per-conn) │  │    (shared)     │  │
│  └──────┬──────┘  └──────┬──────┘  └────────┬────────┘  │
│         │                │                   │          │
│         └───────────┬────┴───────────────────┘          │
│                     │                                   │
│  ┌──────────────────▼──────────────────────────────┐    │
│  │              Server State                        │    │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────────┐   │    │
│  │  │ Document │  │ Op Log   │  │   Clients    │   │    │
│  │  │  (v N)   │  │ [0..N]   │  │  [id -> tx]  │   │    │
│  │  └──────────┘  └──────────┘  └──────────────┘   │    │
│  └──────────────────────────────────────────────────┘   │
│                          │                              │
│  ┌───────────────────────▼──────────────────────────┐   │
│  │              Transform Engine                     │   │
│  │  • Insert vs Insert  • Delete vs Delete          │   │
│  │  • Insert vs Delete  • Delete vs Replace         │   │
│  │  • Insert vs Replace • Replace vs Replace        │   │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

## Roadmap

See [docs/tasks.md](docs/tasks.md) for the full implementation roadmap.

- **Phase 1**: Foundation & Stability ✅
- **Phase 2**: Virtual File System (VFS) 🔜
- **Phase 3**: WASM Plugin System
- **Phase 4**: SDK & CLI Tools

## License

MIT


*readme is generated by ai