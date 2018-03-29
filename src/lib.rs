//! A full-featured router for abstract-ns
//! ======================================
//!
//! [Docs](https://docs.rs/graphql-parser/) |
//! [Github](https://github.com/tailhook/graphql-parser/) |
//! [Crate](https://crates.io/crates/graphql-parser)
//!
//! A router should be an entry point for your application. It solves two
//! purposes:
//!
//! 1. Allow to use different name resolution mechanisms without cluttering
//!    your app with too much generics
//! 2. Optimize performance: we connect to resolvers via channels, so polling
//!    an `ResolveFuture` or `AddrStream` is cheap.
//!
//! # Example
//!
//! A simple resolver you should start with:
//!
//! ```rust
//! # use std::time::Duration;
//! # extern crate tokio_core;
//! extern crate abstract_ns;
//! extern crate ns_router;
//! extern crate ns_std_threaded;
//! use abstract_ns::HostResolve;
//! use ns_router::SubscribeExt;
//!
//! # fn main() {
//! # let core = tokio_core::reactor::Core::new().unwrap();
//! let ns = ns_router::Router::from_config(&ns_router::Config::new()
//!     .set_fallthrough(ns_std_threaded::ThreadedResolver::new()
//!         .null_service_resolver()
//!         .interval_subscriber(Duration::new(1, 0), &core.handle()))
//!     .done(),
//!     &core.handle());
//! # }
//!
//! ```
//!
//! This is a bit verbose, but this means:
//!
//! 1. Use stdlib resolver
//! 2. Do not resolve SRV (stdlib resolver can't do that)
//! 3. Poll for a new address every second (for subscriptions)
//!
//! # Names in Configs
//!
//! You should use [`resolve_auto`] and [`subscribe_many`] for resolving
//! names that come from configuration files. In it's simplest form it
//! accepts a string:
//!
//! ```rust,ignore
//! ns.resolve_auto("localhost:8080")
//! ns.resolve_auto("_xmpp-server._tcp.gmail.com")   // if resolver has SRV
//! ```
//!
//! But you may force specific mode:
//!
//! ```rust.ignore
//! use ns_router::AutoName::*;
//! ns.resolve_auto(Service("localhost"));   // resolve bare `localhost` as SRV
//! ```
//!
//! All the same forms work for `subscribe_many` and `subscribe_many_stream`
//!
//! # Updating Names
//!
//! If you're writing server you might need to react on name changes.
//! `abstract-ns` gives you a way to `subscribe` for the name.
//!
//! Router gives you a little bit more:
//!
//! 1. You can create router with [`from_stream`] or [`updating_config`] and
//!    push configuration updates to router.
//! 2. The [`subscribe_many_stream`] method allows to subsribe to a stream of
//!    names: i.e. every time your user changes list of names to be used for
//!    some service, new names are resolved and applied immediately.
//!
//! Both kinds of updates are delivered to the application as a next update
//! in a stream without any additional code on receiver side.
//!
//! [`resolve_auto`]: struct.Router.html#method.resolve_auto
//! [`subscribe_many`]: struct.Router.html#method.subscribe_many
//! [`subscribe_many_stream`]: struct.Router.html#method.subscribe_many_stream
//! [`from_stream`]: struct.Router.html#method.from_stream
//! [`updating_config`]: struct.Router.html#method.updating_config
//!
//! # Configuring Router
//!
//! To show you an example how router can be configured:
//!
//! ```rust,ignore
//! let cfg = &Config::new()
//!     // resolution of hosts from memory for performance or tests
//!     .add_host(&"localhost".parse().unwrap(),
//!               vec!["127.0.0.1".parse().unwrap()])
//!     // resolve `.consul` names via consul DNS SRV or HTTP
//!     .add_suffix("consul", consul_resolver)
//!     // use stdlib for other things
//!     .set_fallthrough(std_resolver)
//!     .done();
//! ```
//!
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

extern crate abstract_ns;
extern crate async_slot;
extern crate futures;
extern crate tokio_core;
extern crate void;
#[macro_use] extern crate log;
#[macro_use] extern crate quick_error;

mod config;
mod coroutine;
mod fuse;
mod internal;
mod internal_traits;
mod multisubscr;
mod name;
mod router;
mod subscr;
pub mod future;
pub mod subscribe_ext;

pub use router::Router;
pub use config::Config;
pub use name::{AutoName, IntoNameIter};
pub use subscribe_ext::SubscribeExt;

trait AssertTraits: Clone + Send + Sync {}
impl AssertTraits for Router {}
