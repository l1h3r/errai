//! Functions and types related to Errai nodes.

use crate::core::Atom;
use crate::core::ExternalPid;
use crate::core::ExternalRef;
use crate::core::InternalPid;
use crate::core::Term;

// -----------------------------------------------------------------------------
// @error - NodeStartError
// -----------------------------------------------------------------------------

/// The error type returned when failing to start the local node.
#[derive(Debug)]
pub enum NodeStartError {
  /// The node is already running.
  AlreadyStarted(InternalPid),
  /// An arbitrary dependency failed to start.
  UnexpectedTerm(Term),
}

// -----------------------------------------------------------------------------
// @error - NodeStopError
// -----------------------------------------------------------------------------

/// The error type returned when failing to stop the local node.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeStopError {
  /// The local node cannot be stopped.
  Immortal,
  /// The local node is not alive.
  NotAlive,
}

// -----------------------------------------------------------------------------
// @error - NodeNotAlive
// -----------------------------------------------------------------------------

/// The error type returned when performing certain
/// distributed operations on a non-distributed node.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeNotAlive {
  private: (),
}

// -----------------------------------------------------------------------------
// @type - NodeState
// -----------------------------------------------------------------------------

/// The current state of an Errai node.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeState {
  /// All nodes connected to this node.
  Connected,
  /// Nodes connected to this node through normal connections.
  Visible,
  /// Nodes connected to this node through hidden connections.
  Hidden,
  /// Nodes that are known to this node.
  Known,
  /// This node.
  This,
}

// -----------------------------------------------------------------------------
// @type - NodeStartOptions
// -----------------------------------------------------------------------------

/// Options used to configure a distributed node.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeStartOptions {
  /// Enable or disable listening for incoming connections.
  pub listen: bool,
  /// Enable or disable hidden mode.
  pub hidden: bool,
}

// -----------------------------------------------------------------------------
// @type - NodeReply
// -----------------------------------------------------------------------------

/// The response message from a node connection/disconnection attempt.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeReply {
  /// The operation failed.
  Failure,
  /// The operation succeeded.
  Success,
  /// The local node is not alive.
  Ignored,
}

// -----------------------------------------------------------------------------
// @type - NodePing
// -----------------------------------------------------------------------------

/// The response message from a node ping request.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodePing {
  /// The ping request failed.
  Pang,
  /// The ping request succeeded.
  Pong,
}

// -----------------------------------------------------------------------------
// @api - Node
// -----------------------------------------------------------------------------

/// Errai distributed node API.
pub struct Node;

impl Node {
  // ---------------------------------------------------------------------------
  // General API
  // ---------------------------------------------------------------------------

  /// Returns the name of the local node.
  ///
  /// If the node is not alive, `nonode@nohost` is returned.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#node/0>
  pub fn this() -> Atom {
    todo!("this/0")
  }

  /// Returns a list of nodes according to the `state` specified.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#nodes/1>
  pub fn list(_state: NodeState) {
    todo!("list/1")
  }

  // ---------------------------------------------------------------------------
  // General API - Lifecycle
  // ---------------------------------------------------------------------------

  /// Turns a non-distributed node into a distributed node by starting
  /// `net_kernel` and other necessary processes.
  ///
  /// REF: <https://www.erlang.org/doc/apps/kernel/net_kernel.html#start/2>
  ///
  /// # Errors
  ///
  /// Raises [`NodeStartError`] if the node is already running or if any
  /// dependencies fail to start.
  pub fn start(_name: Atom, _opts: NodeStartOptions) -> InternalPid {
    todo!("start/2")
  }

  /// Turns a distributed node into a non-distributed node.
  ///
  /// For other nodes in the network, this is the same as the node going down.
  ///
  /// REF: <https://www.erlang.org/doc/apps/kernel/net_kernel.html#stop/0>
  ///
  /// # Errors
  ///
  /// Raises [`NodeStopError`] if the node was not started using [`Node::start`]
  /// or if the local node is not alive.
  pub fn stop() {
    todo!("stop/0")
  }

  // ---------------------------------------------------------------------------
  // General API - Distributed Errai
  // ---------------------------------------------------------------------------

  /// Returns `true` if the local node is alive (that is, if the node can be
  /// part of a distributed system), otherwise `false`.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#is_alive/0>
  pub fn alive() -> bool {
    todo!("alive/0")
  }

  /// Establishes a connection to `node`.
  ///
  /// REF: <https://www.erlang.org/doc/apps/kernel/net_kernel.html#connect_node/1>
  pub fn connect(_node: Atom) -> NodeReply {
    todo!("connect/1")
  }

  /// Forces the disconnection of a node.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#disconnect_node/1>
  pub fn disconnect(_node: Atom) -> NodeReply {
    todo!("disconnect/1")
  }

  /// Tries to set up a connection to `node`.
  ///
  /// REF: <https://www.erlang.org/doc/apps/kernel/net_adm.html#ping/1>
  pub fn ping(_node: Atom) -> NodePing {
    todo!("ping/1")
  }

  /// Monitors the status of the node `node`.
  ///
  /// Making several calls to [`Node::monitor`] for the same `node` is not an error;
  /// it results in as many independent monitoring instances.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#monitor_node/2>
  ///
  /// # Errors
  ///
  /// Raises [`NodeNotAlive`] if the local node is not alive.
  pub fn monitor(_node: Atom) {
    todo!("monitor/1")
  }

  /// Demonitors the status of the node `node`.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#monitor_node/2>
  ///
  /// # Errors
  ///
  /// Raises [`NodeNotAlive`] if the local node is not alive.
  pub fn demonitor(_node: Atom) {
    todo!("demonitor/1")
  }

  /// Returns the magic cookie of the local node if the node is alive, otherwise the atom `nocookie`.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#get_cookie/0>
  pub fn get_cookie() -> Atom {
    todo!("get_cookie/0")
  }

  /// Sets the magic cookie of the local node to the atom `cookie`.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#set_cookie/1>
  ///
  /// # Errors
  ///
  /// Raises [`NodeNotAlive`] if the local node is not alive.
  pub fn set_cookie(_cookie: Atom) {
    todo!("set_cookie/1")
  }

  /// Spawns a new process on `node` to handle `future`.
  ///
  /// Returns the external PID, or `None` if `node` does not exist.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#spawn/2>
  pub async fn spawn<T>(_node: Atom, _future: T) -> Option<ExternalPid>
  where
    T: Future<Output = ()> + Send + 'static,
  {
    todo!("spawn/2")
  }

  /// Spawns a new atomically linked process on `node` to handle `future`.
  ///
  /// Returns the external PID, or `None` if `node` does not exist.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#spawn_link/2>
  pub async fn spawn_link<T>(_node: Atom, _future: T) -> Option<ExternalPid>
  where
    T: Future<Output = ()> + Send + 'static,
  {
    todo!("spawn_link/2")
  }

  /// Spawns a new atomically monitored process on `node` to handle `future`.
  ///
  /// Returns the external PID and monitor reference, or `None` if `node` does not exist.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#spawn_monitor/2>
  pub async fn spawn_monitor<T>(_node: Atom, _future: T) -> Option<(ExternalPid, ExternalRef)>
  where
    T: Future<Output = ()> + Send + 'static,
  {
    todo!("spawn_monitor/2")
  }
}
