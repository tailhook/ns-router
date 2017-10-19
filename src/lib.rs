//! A full-featured router for abstract-ns
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

extern crate abstract_ns;
extern crate futures;
extern crate tokio_core;
extern crate void;
#[macro_use] extern crate log;
#[macro_use] extern crate quick_error;

mod config;
mod coroutine;
mod internal;
mod internal_traits;
mod multisubscr;
mod name;
mod router;
mod slot;  // TODO(tailhook) it should be added to futures-rs
mod subscr;
pub mod future;
pub mod subscribe_ext;

pub use router::Router;
pub use config::Config;
pub use name::AutoName;
pub use subscribe_ext::SubscribeExt;

trait AssertTraits: Clone + Send + Sync {}
impl AssertTraits for Router {}
