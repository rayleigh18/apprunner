//! API Mask reverse proxy module.
//!
//! Provides a local HTTP proxy that injects configured headers before
//! forwarding requests to an upstream target URL. Used to hide API keys
//! from agents/tools that connect to localhost.

pub mod log;
pub mod proxy;
