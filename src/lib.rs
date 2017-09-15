extern crate abstract_ns;
extern crate futures;

mod slot;  // TODO(tailhook) it should be added to futures-rs
mod router;
mod abstract_impl;

pub use router::Router;
