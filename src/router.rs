use std::fmt;
use std::sync::Arc;

use abstract_ns::{Name, Resolve, HostResolve, Subscribe, HostSubscribe};
use abstract_ns::{Address, Error};
use futures::{Stream, Future};
use futures::future::{empty};
use futures::stream::{once};
use futures::sync::oneshot;
use futures::sync::mpsc::{unbounded, UnboundedSender};
use tokio_core::reactor::Handle;
use void::Void;

use config::Config;
use coroutine::{ResolverFuture};
use future::{AddrStream, ResolveFuture, HostStream, ResolveHostFuture};
use future::{UpdateSink};
use internal::{fail, Request};
use multisubscr::MultiSubscr;
use name::{AutoName, InternalName, IntoNameIter};
use async_slot as slot;
use subscr::Wrapper;

/// An actual router class
///
/// Note: when router is shut down (when config stream is closed), all futures
/// and subscriptions are canceled.
///
/// *Since 0.1.3*: when all instances of Router are dropped the underlying
/// futures and streams continue to work, until all of them finish (for
/// subscriptions it means subscriber is dropped). In previous versions this
/// dropped all requests immediately. This was common source of confusion and
/// we consider this a bug. If you want to force close all futures and
/// subscriptions create a router with `from_stream` or `updating_config` and
/// send EOS on stream or drop `UpdatingSink` respectively.
#[derive(Debug, Clone)]
pub struct Router {
    requests: UnboundedSender<Request>,
}


impl Router {

    /// Create a router for a static config
    pub fn from_config(config: &Arc<Config>, handle: &Handle) -> Router {
        let (tx, rx) = unbounded();
        handle.spawn(ResolverFuture::new(
            once(Ok(config.clone())).chain(empty().into_stream()),
            rx, &handle));
        Router {
            requests: tx,
        }
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
        let (tx, rx) = unbounded();
        handle.spawn(ResolverFuture::new(stream, rx, &handle));
        Router {
            requests: tx,
        }
    }

    /// Create a router and update channel
    ///
    /// Note: router is shut down when `UpdateSink` is dropped. So keep
    /// it somewhere so you can update config.
    pub fn updating_config(config: &Arc<Config>, handle: &Handle)
        -> (Router, UpdateSink)
    {
        let (ctx, crx) = slot::channel();
        let stream = once(Ok(config.clone())).chain(crx)
            .map_err(|_| unreachable!());
        let (tx, rx) = unbounded();
        handle.spawn(ResolverFuture::new(stream, rx, &handle));
        return (
            Router {
                requests: tx,
            },
            UpdateSink(ctx),
        );
    }

    pub(crate) fn _subscribe_stream<S>(&self,
        stream: S, tx: slot::Sender<Address>)
        where S: Stream<Item=Vec<InternalName>> + Send + 'static,
              S::Error: fmt::Display,
    {
        self.requests.unbounded_send(
            Request::Task(Wrapper::wrap_send(MultiSubscr::new(stream, tx))))
            // can't do anything when resolver is down, (no error in stream)
            // but this will shut down stream which will be visible
            // for the appplication, which is probably shutting down anyway
            .map_err(|_| debug!("Stream subscription when resolver is down"))
            .ok();
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
        self._subscribe_stream(
            once(Ok::<_, Void>(lst)).chain(empty().into_stream()), tx);
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
    #[deprecated(since="0.1.1", note="use subscribe_stream()")]
    pub fn subscribe_many_stream<'x, S>(&self, stream: S, default_port: u16)
        -> AddrStream
        where S: Stream + Send + 'static,
              S::Item: IntoIterator,
              S::Error: fmt::Display,
              <S::Item as IntoIterator>::Item: Into<AutoName<'x>>,
    {
        let (tx, rx) = slot::channel();
        self._subscribe_stream(stream.map(move |iter| {
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
    pub fn subscribe_stream<S>(&self, stream: S, default_port: u16)
        -> AddrStream
        where S: Stream + Send + 'static,
              S::Error: fmt::Display,
              for<'x> S::Item: IntoNameIter<'x>,
    {
        let (tx, rx) = slot::channel();
        self._subscribe_stream(stream.map(move |iter| {
            let mut lst = Vec::new();
            for addr in iter.into_name_iter() {
                match addr.into().parse(default_port) {
                    Ok(x) => lst.push(x),
                    Err(e) => {
                        warn!("Error parsing name: {}", e);
                        //unimplemented!();
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
                match self.requests.unbounded_send(
                    Request::ResolveHostPort(name.clone(), port, tx))
                {
                    Ok(()) => {}
                    Err(e) => match e.into_inner() {
                        Request::ResolveHostPort(name, _, tx) => {
                            fail(&name, tx, Error::TemporaryError(
                                "Resolver is down".into()));
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Ok(InternalName::Service(name)) => {
                match self.requests.unbounded_send(
                    Request::Resolve(name.clone(), tx))
                {
                    Ok(()) => {}
                    Err(e) => match e.into_inner() {
                        Request::Resolve(name, tx) => {
                            fail(&name, tx, Error::TemporaryError(
                                "Resolver is down".into()));
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Ok(InternalName::Addr(addr)) => {
                tx.send(Ok(addr.into())).ok();
            }
            Err(e) => {
                tx.send(Err(e.into())).ok();
            }
        }
        ResolveFuture(rx)
    }

}

impl HostResolve for Router {
    type HostFuture = ResolveHostFuture;
    fn resolve_host(&self, name: &Name) -> ResolveHostFuture {
        let (tx, rx) = oneshot::channel();
        match self.requests.unbounded_send(
            Request::ResolveHost(name.clone(), tx))
        {
            Ok(()) => {}
            Err(e) => match e.into_inner() {
                Request::ResolveHost(name, tx) => {
                    fail(&name, tx, Error::TemporaryError(
                        "Resolver is down".into()));
                }
                _ => unreachable!(),
            }
        }
        ResolveHostFuture(rx)
    }
}

impl Resolve for Router {
    type Future = ResolveFuture;
    fn resolve(&self, name: &Name) -> ResolveFuture {
        let (tx, rx) = oneshot::channel();
        match self.requests.unbounded_send(
            Request::Resolve(name.clone(), tx))
        {
            Ok(()) => {}
            Err(e) => match e.into_inner() {
                Request::Resolve(name, tx) => {
                    fail(&name, tx, Error::TemporaryError(
                        "Resolver is down".into()));
                }
                _ => unreachable!(),
            }
        }
        ResolveFuture(rx)
    }

}

impl HostSubscribe for Router {
    type HostError = Void;
    type HostStream = HostStream;
    fn subscribe_host(&self, name: &Name) -> HostStream {
        let (tx, rx) = slot::channel();
        self.requests.unbounded_send(
            Request::HostSubscribe(name.clone(), tx))
            // can't do anything when resolver is down, (no error in stream)
            // but this will shut down stream which will be visible
            // for the appplication, which is probably shutting down anyway
            .map_err(|_| debug!("Subscription for {} when resolver is down",
                name))
            .ok();
        HostStream(rx)
    }
}

impl Subscribe for Router {
    type Error = Void;
    type Stream = AddrStream;
    fn subscribe(&self, name: &Name) -> AddrStream {
        let (tx, rx) = slot::channel();
        self.requests.unbounded_send(
            Request::Subscribe(name.clone(), tx))
            // can't do anything when resolver is down, (no error in stream)
            // but this will shut down stream which will be visible
            // for the appplication, which is probably shutting down anyway
            .map_err(|_| debug!("Subscription for {} when resolver is down",
                name))
            .ok();
        AddrStream(rx)
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod type_test {
    use std::sync::Arc;
    use futures::Stream;
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

    #[derive(Debug)]
    enum MyName {
        Auto(String),
        Service(String),
    }

    #[derive(Debug)]
    struct List1(Arc<Vec<MyName>>);

    #[derive(Debug)]
    struct List2(Arc<Vec<MyName>>);

    impl<'a> IntoIterator for &'a List1 {
        type Item = AutoName<'a>;
        type IntoIter = ::std::iter::Map<::std::slice::Iter<'a, MyName>, fn(&'a MyName) -> AutoName<'a>>;
        fn into_iter(self) -> Self::IntoIter {
            self.0.iter().map(|x| x.into())
        }
    }

    impl<'a> IntoIterator for &'a List2 {
        type Item = &'a MyName;
        type IntoIter = ::std::slice::Iter<'a, MyName>;
        fn into_iter(self) -> Self::IntoIter {
            self.0.iter()
        }
    }

    impl<'a> Into<AutoName<'a>> for &'a MyName {
        fn into(self) -> AutoName<'a> {
            match *self {
                MyName::Auto(ref x) => AutoName::Auto(x),
                MyName::Service(ref x) => AutoName::Service(x),
            }
        }
    }

    fn test_subscribe_vec<S>(r: &Router, s: S)
        where S: Stream<Item=Vec<String>, Error=String> + Send + 'static
    {
        r.subscribe_stream(s, 80);
    }

    fn test_subscribe_vec_custom<S>(r: &Router, s: S)
        where S: Stream<Item=Vec<MyName>, Error=String> + Send + 'static
    {
        r.subscribe_stream(s, 80);
    }

    fn test_subscribe_arc_vec1<S>(r: &Router, s: S)
        where S: Stream<Item=List1, Error=String> + Send + 'static
    {
        r.subscribe_stream(s, 80);
    }

    fn test_subscribe_arc_vec2<S>(r: &Router, s: S)
        where S: Stream<Item=List2, Error=String> + Send + 'static
    {
        r.subscribe_stream(s, 80);
    }
}
