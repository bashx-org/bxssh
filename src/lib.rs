// Library interface for WASM bindings
#![allow(clippy::missing_safety_doc)]

#[cfg(target_arch = "wasm32")]
pub mod wasm_exports;

// Re-export core modules for library usage
pub mod ssh_client;
pub mod config;

#[cfg(target_arch = "wasm32")]
pub mod wasm_ssh;

#[cfg(target_arch = "wasm32")]
pub mod ssh_protocol;

#[cfg(not(target_arch = "wasm32"))]
pub mod ssh_impl;

#[cfg(not(target_arch = "wasm32"))]
pub mod native;

#[cfg(not(target_arch = "wasm32"))]
pub mod key_manager;

#[cfg(not(target_arch = "wasm32"))]
pub mod terminal;

#[cfg(not(target_arch = "wasm32"))]
pub mod cli_terminal;

// WASM-specific exports
#[cfg(target_arch = "wasm32")]
pub use wasm_exports::*;