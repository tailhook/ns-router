use std::fmt;
use std::sync::Arc;

use abstract_ns::{Name, Resolve, ResolveHost, Subscribe, HostSubscribe};
use futures::{Stream, Future};
use futures::future::{empty};
use futures::stream::{once};
use futures::sync::oneshot;
use internal::Table;
use tokio_core::reactor::Handle;
use void::Void;

use config::Config;
use future::{AddrStream, ResolveFuture, HostStream, ResolveHostFuture};
use future::{UpdateSink};
use name::{AutoName, InternalName};
use slot;


/// An actual router class
///
/// Note: when router is shut down (when config stream is closed), all futures
/// and subscriptions are canceled. We'll probably do the same when all
/// router instances are dropped too.
#[derive(Debug, Clone)]
pub struct Router(pub(crate) Arc<Table>);


impl Router {

    /// Create a router for a static config
    pub fn from_config(config: &Arc<Config>, handle: &Handle) -> Router {
        Router(Table::new(
            once(Ok(config.clone())).chain(empty().into_stream()),
            handle))
    }

    /// Create a router with updating config
    ///
    /// Note: router is defunctional until first config is received in a
    /// stream. By defunctional we mean that every request will wait, until
    /// configured.
    ///
    /// Note 2: when stream is closed router is shut down, so usually the
    /// stream must be infinite.
    pub fn from_stream<S>(stream: S, handle: &Handle) -> Router
        where S: Stream<Item=Arc<Config>, Error=Void> + 'static
    {
        Router(Table::new(stream, handle))
    }

    /// Create a router and update channel
    ///
    /// Note: router is shut down when `UpdateSink` is dropped. So keep
    /// it somewhere so you can update config.
    pub fn updating_config(config: &Arc<Config>, handle: &Handle)
        -> (Router, UpdateSink)
    {
        let (tx, rx) = slot::channel();
        let stream = once(Ok(config.clone())).chain(rx)
            .map_err(|_| unreachable!());
        return (Router(Table::new(stream, handle)), UpdateSink(tx));
    }

    /// Subscribes to a list of names
    ///
    /// This is intended to keep list of services in configuration file,
    /// like this (yaml):
    ///
    /// ```yaml
    /// addresses:
    /// - example.org:8080
    /// - _my._svc.example.org  # SVC record
    /// - example.net           # default port
    /// ```
    ///
    /// You can also specify a way to resolve the service by providing
    /// iterator over `AutoName` instances instead of plain `&str` (both are
    /// accepted in this method).
    pub fn subscribe_many<'x, I>(&self, iter: I, default_port: u16)
        -> AddrStream
        where I: IntoIterator,
              I::Item: Into<AutoName<'x>>,
    {
        let (tx, rx) = slot::channel();
        let mut lst = Vec::new();
        for addr in iter {
            match addr.into().parse(default_port) {
                Ok(x) => lst.push(x),
                Err(e) => {
                    warn!("Error parsing name: {}", e);
                }
            }
        }
        self.0.subscribe_stream(
            once(Ok::<_, Void>(lst)).chain(empty().into_stream()),
            tx);
        AddrStream(rx)
    }

    /// Subscribes to a stream that yields lists of names
    ///
    /// See the description of [`subscribe_many`](#tymethod.subscribe_many)
    /// for the description of the list of names that must be yielded from
    /// the stream.
    ///
    /// Note: this is meant for configuration update scenario. I.e. when
    /// configuration is reloaded and new list of names is received, it
    /// should be pushed to this stream. The items received in the stream
    /// are non-cumulative and replace previous list.
    ///
    /// Note 2: If stream is errored or end-of-stream reached, this means
    /// name is not needed any more and its `AddrStream` will be shut down,
    /// presumably shutting down everything that depends on it.
    pub fn subscribe_many_stream<'x, S>(&self, stream: S, default_port: u16)
        -> AddrStream
        where S: Stream + 'static,
              S::Item: IntoIterator,
              S::Error: fmt::Display,
              <S::Item as IntoIterator>::Item: Into<AutoName<'x>>,
    {
        let (tx, rx) = slot::channel();
        self.0.subscribe_stream(stream.map(move |iter| {
            let mut lst = Vec::new();
            for addr in iter {
                match addr.into().parse(default_port) {
                    Ok(x) => lst.push(x),
                    Err(e) => {
                        warn!("Error parsing name: {}", e);
                    }
                }
            }
            lst
        }), tx);
        AddrStream(rx)
    }
    /// Resolve a string or other things into an address
    ///
    /// See description of [`subscribe_many`] to find out how names are parsed
    ///
    /// See [`AutoName`] for supported types
    ///
    /// [`subscribe_many`]: #tymethod.subscribe_many
    /// [`AutoName`]:
    pub fn resolve_auto<'x, N: Into<AutoName<'x>>>(&self,
        name: N, default_port: u16)
        -> ResolveFuture
    {
        let (tx, rx) = oneshot::channel();
        match name.into().parse(default_port) {
            Ok(InternalName::HostPort(name, port)) => {
                self.0.resolve_host_with_port(&name, port, tx)
            }
            Ok(InternalName::Service(name)) => {
                self.0.resolve(&name, tx)
            }
            Err(e) => {
                tx.send(Err(e.into())).ok();
            }
        }
        ResolveFuture(rx)
    }

}

impl ResolveHost for Router {
    type FutureHost = ResolveHostFuture;
    fn resolve_host(&self, name: &Name) -> ResolveHostFuture {
        let (tx, rx) = oneshot::channel();
        self.0.resolve_host(name, tx);
        ResolveHostFuture(rx)
    }
}

impl Resolve for Router {
    type Future = ResolveFuture;
    fn resolve(&self, name: &Name) -> ResolveFuture {
        let (tx, rx) = oneshot::channel();
        self.0.resolve(name, tx);
        ResolveFuture(rx)
    }

}

impl HostSubscribe for Router {
    type Error = Void;
    type HostStream = HostStream;
    fn subscribe_host(&self, name: &Name) -> HostStream {
        let (tx, rx) = slot::channel();
        self.0.subscribe_host(name, tx);
        HostStream(rx)
    }
}

impl Subscribe for Router {
    type Error = Void;
    type Stream = AddrStream;
    fn subscribe(&self, name: &Name) -> AddrStream {
        let (tx, rx) = slot::channel();
        self.0.subscribe(name, tx);
        AddrStream(rx)
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod type_test {
    use name::AutoName;
    use super::Router;

    fn test_vec_string(r: &Router, v: Vec<String>) {
        r.subscribe_many(&v, 1);
    }

    fn test_vec_str(r: &Router, v: Vec<&str>) {
        r.subscribe_many(&v, 1);
    }

    fn test_vec_auto(r: &Router, v: Vec<AutoName>) {
        r.subscribe_many(v, 1);
    }

    fn test_map_auto(r: &Router, v: Vec<&str>) {
        r.subscribe_many(v.into_iter().map(AutoName::Auto), 1);
    }
}
