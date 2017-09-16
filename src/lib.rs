//! A full-featured router for abstract-ns
//!
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

extern crate abstract_ns;
extern crate futures;
extern crate void;

mod config;
mod slot;  // TODO(tailhook) it should be added to futures-rs
mod router;
mod abstract_impl;

pub use router::Router;
pub use config::Config;
