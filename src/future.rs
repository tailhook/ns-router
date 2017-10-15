//! Futures and streams returned from router
//!
use abstract_ns::{Name, IpList, Address, Error};
use abstract_ns::{Resolve, ResolveHost, Subscribe, HostSubscribe};
use futures::sync::oneshot;
use futures::{Future, Async, Stream};
use void::Void;

use slot;
use router::Router;

/// A future returned from `Router::resolve_host`
#[derive(Debug)]
pub struct ResolveHostFuture(oneshot::Receiver<Result<IpList, Error>>);

/// A future returned from `Router::resolve`
#[derive(Debug)]
pub struct ResolveFuture(pub(crate) oneshot::Receiver<Result<Address, Error>>);

/// A stream returned from `Router::host_subscribe`
#[derive(Debug)]
pub struct HostStream(slot::Receiver<IpList>);

/// A stream returned from `Router::subscribe`
#[derive(Debug)]
pub struct AddrStream(pub(crate) slot::Receiver<Address>);


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

impl Future for ResolveHostFuture {
    type Item = IpList;
    type Error = Error;
    #[inline(always)]
    fn poll(&mut self) -> Result<Async<IpList>, Error> {
        match self.0.poll().map_err(|e| Error::TemporaryError(e.into()))? {
            Async::NotReady => Ok(Async::NotReady),
            Async::Ready(Ok(r))  => Ok(Async::Ready(r)),
            Async::Ready(Err(e))  => Err(e),
        }
    }
}

impl Future for ResolveFuture {
    type Item = Address;
    type Error = Error;
    #[inline(always)]
    fn poll(&mut self) -> Result<Async<Address>, Error> {
        match self.0.poll().map_err(|e| Error::TemporaryError(e.into()))? {
            Async::NotReady => Ok(Async::NotReady),
            Async::Ready(Ok(r))  => Ok(Async::Ready(r)),
            Async::Ready(Err(e))  => Err(e),
        }
    }
}

impl Stream for HostStream {
    type Item = IpList;
    type Error = Void;
    #[inline(always)]
    fn poll(&mut self) -> Result<Async<Option<IpList>>, Void> {
        match self.0.poll() {
            Ok(r) => Ok(r),
            Err(_) => Ok(Async::Ready(None)),
        }
    }
}

impl Stream for AddrStream {
    type Item = Address;
    type Error = Void;
    #[inline(always)]
    fn poll(&mut self) -> Result<Async<Option<Address>>, Void> {
        match self.0.poll() {
            Ok(r) => Ok(r),
            Err(_) => Ok(Async::Ready(None)),
        }
    }
}
