//! Rust SDK for Ante.
//!
//! A [`Client`] is one connection to an Ante host: it sends [`protocol::Op`]s
//! and receives [`protocol::EventMsg`]s. Obtain one with [`connect`] against
//! an [`Endpoint`] — where a host is reachable — or, inside a process that
//! hosts sessions itself, from that host directly. Both paths yield the same
//! type, and the in-process channel carries the same wire types the remote
//! codecs serialize, so nothing a client can observe differs between them.
//!
//! The [`claude`] module is unrelated to the Ante protocol: it drives Claude
//! Code as a child process.

pub mod claude;
mod client;
mod connect;
mod endpoint;
pub mod stdio;

pub use ante_protocol_shape as protocol;
pub use client::{Client, Closed, EventReceiver, OpSender};
pub use connect::{ConnectError, ConnectOptions, connect};
pub use endpoint::{Endpoint, EndpointParseError};
