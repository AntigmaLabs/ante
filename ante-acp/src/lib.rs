//! Agent Client Protocol (ACP) agent for Ante.
//!
//! `ante-acp` is the process an ACP client (Zed, a JetBrains IDE, ...) launches
//! and talks to over stdio. It drives an installed `ante` binary and
//! translates between the two protocols.

pub mod agent;
pub mod ante_bin;
pub mod session;
