# Errai

An Erlang-inspired actor runtime for Rust, built on Tokio.

Errai brings Erlang's proven concurrency model to Rust: lightweight processes that communicate through message passing, isolated failure domains with supervision trees, and transparent distributed communication.

## Key Features

- **Lightweight processes**: Spawn millions of concurrent actors, each with isolated state and independent failure handling
- **Message passing**: Type-safe async communication between processes
- **Links and monitors**: Build fault-tolerant systems with bidirectional links and one-way monitoring
- **Supervision trees**: Automatic process restart and cascading failure handling
- **Process registry**: Register processes by name for location-transparent messaging

## Quick Start

```rust
use errai::core::Exit;
use errai::erts::{Process, DynMessage};
use errai::init;

init::block_on(async move {
  let root = Process::this();

  let pid = Process::spawn(async move {
    if let DynMessage::Term(name) = Process::receive_any().await {
      println!("Hello, {}!", name);
      Process::exit(root, Exit::NORMAL);
    } else {
      println!("Received unexpected message");
    }
  });

  Process::send(pid, "World");

  if let DynMessage::Exit(exit) = Process::receive_any().await {
    println!("Received exit signal: {:?}", exit);
  } else {
    println!("Received unexpected message");
  }
});
```
