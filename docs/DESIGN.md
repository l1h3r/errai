# Design Overview

## Core Principles

### Process

- Lightweight actors that communicate using asynchronous signals.
- Small memory footprint and low scheduling overhead.
- Globally addressed via a unique Process ID (PID).

### Name Registration

A process can be addressed using a locally registered name. The name is an `atom` and is automatically unregistered when the process terminates.

- `Process::register(pid, name)`
  - Associates a `name` with a `pid`.

- `Process::unregister(name)`
  - Removes the association between a `name` and its corresponding `pid`.

- `Process::whereis(name) -> Option<PID>`
  - Returns the `pid` associated with `name`, or `None` if not found.

- `Process::registered() -> Vec<atom>`
  - Returns a list of all registered names.

### Termination

A process **always** terminates with an exit reason, which can be any `term` or the special values `normal` and `killed`.

- By default, a process terminates with `normal`, indicating successful execution.
- A process may terminate with a custom reason `<term>` in case of failure.
- A process can also terminate with the reason `killed` after an explicit call to `Process::exit(pid, reason)` where `reason` is `killed`.

### Async Signals

All communication between processes is done asynchronously through signals. The most common signal is the `send` signal, which can be sent using `Process::send`. Messages can be received via `Process::{receive, receive_exact, receive_any}` based on the match requirements.

#### Send/Recv Example

```rust
// Sending a message with "Process A"
Process::send(<process B pid>, "ping");

// Receiving a message with "Process B"
assert_eq!(Process::receive::<&'static str>().await, Message::Term(Box::new("ping")));
// OR
assert_eq!(Process::receive_exact::<&'static str>().await, Box::new("ping"));
// OR
assert_eq!(Process::receive_any().await, DynMessage::Term(Term::new("ping")));
```

### Links/Monitors

A process can be "linked" to another using `Process::link(..)`. Linked processes provide bidirectional failure observation and crash propagation. If one of the linked processes terminates, it sends an `exit` signal to the other, populated with the exit reason of the terminating process. Links can be removed using `Process::unlink(..)`.

Monitors, on the other hand, offer unidirectional failure observation. When a monitored process terminates, it sends a `down` signal to the monitoring process. The `down` signal is purely observational and does not terminate the monitoring process. Monitors are created using `Process::monitor(..)` and removed with `Process::demonitor(..)`.

### Error Handling

### Signal Reference

- **`exit`**
  - Sent when calling `Process::exit`.
  - Sent when a linked process terminates.

- **`down`**
  - Sent when a monitored process terminates.

- **`message`**
  - Sent when calling `Process::send`.

- **`link`**
  - Sent when calling `Process::link`.

- **`unlink`**
  - Sent when calling `Process::unlink`.

- **`monitor`**
  - Sent when calling `Process::monitor`.

- **`demonitor`**
  - Sent when calling `Process::demonitor`.
  - Sent when a process monitoring another process terminates.
