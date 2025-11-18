# An experiment on distributed workspace engine

**A distributed, deterministic workspace engine that synchronizes code edits across humans and AI agents, maintains a bounded operation log, resolves concurrent edits, and exposes real-time snapshots and context for tools.**

A backend engine that multiple humans + AI agents can connect to,

**edit a codebase through,
and always see the same synchronized workspace evolve in real-time**

- Everyone connects to it.  
- Everyone sees the same workspace.  
- Everyone’s actions funnel through it.  
- AI agents have the same view as humans.  
- The system resolves conflicts + maintains causality + logs history.

That’s the end product.

Current demo includes,

**A demo where we run two terminals + one AI agent**

1. Start node A
    
2. Start node B
    
3. They sync
    
4. Open client 1 on node A
    
5. Open client 2 on node B
    
6. Both see the same workspace
    
7. Client 1 types code
    
8. Client 2 sees it immediately
    
9. An AI agent attaches
    
10. It reads the last 100 ops
    
11. It generates a patch
    
12. Patch applies and everyone sees it
    
13. If both edit at once, the merge rule resolves it deterministically
    
14. A snapshot is produced
    
15. You can replay the last 50 ops deterministically and get the same result


## To see the demo

Try to run:

```
  cargo run -p server
```

And to run the tests finally,

```
  cargo run -p tests
```
