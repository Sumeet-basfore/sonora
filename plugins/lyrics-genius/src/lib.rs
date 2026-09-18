//! Sonora Genius lyrics provider (WASM).
//!
//! Pure client logic lives in [`client`] (host-independent, unit-tested).
//! The WASM ABI in [`abi`] is compiled for `wasm32-unknown-unknown` only.

pub mod client;

#[cfg(target_arch = "wasm32")]
mod abi;
