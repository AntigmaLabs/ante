//! Rust SDK for Ante.
//!
//! A [`Client`] is one connection to an Ante host: it sends [`protocol::Op`]s
//! and receives [`protocol::EventMsg`]s. Obtain one with [`connect`] against
//! an [`Endpoint`] — where a host is reachable — or, inside a process that
//! hosts sessions itself, from that host directly. Both paths yield the same
//! type and carry the same wire types, so a client observes no difference.

mod client;
mod connect;
mod endpoint;

pub use ante_protocol_shape as protocol;
pub use client::{Client, Closed, EventReceiver, OpSender};
pub use connect::{ConnectError, ConnectOptions, connect};
pub use endpoint::{Endpoint, EndpointParseError};
