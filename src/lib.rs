//! A full-featured router for abstract-ns
//!
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

extern crate abstract_ns;
extern crate futures;
extern crate tokio_core;
extern crate void;
#[macro_use] extern crate log;

mod abstract_impl;
mod cell;
mod config;
mod coroutine;
mod internal;
mod router;
mod slot;  // TODO(tailhook) it should be added to futures-rs

pub use router::Router;
pub use config::Config;
