//! The app BFF service library face: configuration, the shared runtime
//! state, and the REST transport. The binary (`main.rs`) is a thin
//! assembly entry over this surface.

pub mod config;
pub mod server;
pub mod services;
pub mod state;
