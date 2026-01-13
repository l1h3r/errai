# Errai - A BEAM-inspired runtime

A BEAM-inspired framework for building fast, scalable, and fault-tolerant distributed systems.

## Key Features

- **Processes**: Lightweight actors with unique Process IDs (PIDs), allowing efficient asynchronous communication.
- **Name Registration**: Processes can be addressed by locally registered names (atoms).
- **Termination**: Processes always terminate with an exit reason (`normal`, `killed`, or a custom term).
- **Async Signals**: Communication between processes is done through asynchronous messages (`send`, `exit`, `link`, `down`, etc.).
- **Links & Monitors**: Provides bidirectional (links) and unidirectional (monitors) failure observation to propagate crashes.
