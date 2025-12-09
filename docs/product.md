# Dist-Space: The Multiplayer Operating System for Code

## 1. Product Vision
**Dist-Space** is a distributed, deterministic runtime designed to be the backbone of the next generation of software development. It transforms the codebase from a static set of files on a disk into a **live, collaborative entity** where humans and AI agents interact in real-time with zero latency.

Unlike traditional collaboration tools that simply sync text, Dist-Space acts as a **runtime environment**—managing state, execution, and history with mathematical precision.

---

## 2. Core Pillars

### The Live Virtual File System (VFS)
*   **Concept**: A memory-first file system that exists simultaneously on all connected clients.
*   **Capability**: Supports real-time creation, deletion, and modification of files and directories.
*   **Guarantee**: **Conflict-Free**. Utilizing Operational Transformation (OT), the system ensures that concurrent edits from multiple sources (e.g., a human typing and an AI refactoring) are merged deterministically without "merge conflicts."

### Embedded AI Compute (WASM)
*   **Concept**: AI agents are not external API calls; they are first-class citizens running *inside* the environment.
*   **Capability**: Users can deploy AI plugins (linters, auto-fixers, junior dev bots) compiled to WebAssembly (WASM).
*   **Guarantee**: **Sandboxed Safety**. Agents run with strict permissions, able to analyze and edit code at memory speed but isolated from the host system's sensitive resources.

### Deterministic State & Time Travel
*   **Concept**: The entire workspace state is a function of its operation log.
*   **Capability**: "Time Travel" debugging. Users can instantly revert the workspace to any previous state (Version 100 -> Version 50) or replay the exact sequence of edits that led to a bug.
*   **Guarantee**: **Auditability**. Every change is signed and logged. You know exactly *who* (human or agent) changed *what* and *when*.

### Universal Headless API
*   **Concept**: Dist-Space is "Headless." It has no UI of its own but powers any interface.
*   **Capability**: Connect via standardized protocols (WebSocket/gRPC).
*   **Use Cases**:
    *   **VS Code Extension**: Developers code locally while syncing to the mesh.
    *   **Web IDEs**: Browser-based editors for instant collaboration.
    *   **CI/CD**: Pipelines connect as "clients" to run tests on live code.

---

## 3. Success Criteria
The product is successful when:
1.  **Reliability**: It can run for weeks without crashing or corrupting data, handling thousands of concurrent operations.
2.  **Performance**: Latency between a human typing and an agent seeing the change is < 50ms.
3.  **Interoperability**: A human on VS Code and an AI agent in the cloud can edit the same file simultaneously without overwriting each other.

---

## 4. Non-Goals
*   **Building a UI**: We are not building a text editor. We are building the *engine* that powers text editors.
*   **Language Specificity**: The core engine is language-agnostic. It treats code as text/tree structures, not specific to Rust or JS.
