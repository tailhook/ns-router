//! A full-featured router for abstract-ns
//!
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

extern crate abstract_ns;
extern crate futures;
extern crate tokio_core;
extern crate void;
#[macro_use] extern crate log;
#[macro_use] extern crate quick_error;

mod cell;
mod config;
mod coroutine;
mod internal;
mod internal_traits;
mod router;
mod slot;  // TODO(tailhook) it should be added to futures-rs
mod subscr;
mod name;
pub mod future;

pub use router::Router;
pub use config::Config;
pub use name::AutoName;
