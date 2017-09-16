use std::net::IpAddr;

use abstract_ns::{Name, Address, Error};
use abstract_ns::{Resolve, ResolveHost, Subscribe, HostSubscribe};
use futures::sync::oneshot;
use futures::{Future, Async, Stream};
use void::Void;

use slot;
use router::Router;

#[derive(Debug)]
pub struct ResolveHostFuture(oneshot::Receiver<Result<Vec<IpAddr>, Error>>);

#[derive(Debug)]
pub struct ResolveFuture(oneshot::Receiver<Result<Address, Error>>);

#[derive(Debug)]
pub struct HostStream(slot::Receiver<Vec<IpAddr>>);

#[derive(Debug)]
pub struct AddrStream(slot::Receiver<Address>);


impl ResolveHost for Router {
    type FutureHost = ResolveHostFuture;
    fn resolve_host(&self, name: &Name) -> ResolveHostFuture {
        unimplemented!();
    }
}

impl Resolve for Router {
    type Future = ResolveFuture;
    fn resolve(&self, name: &Name) -> ResolveFuture {
        unimplemented!();
    }

}

impl HostSubscribe for Router {
    type Error = Void;
    type HostStream = HostStream;
    fn subscribe_host(&self, name: &Name) -> HostStream {
        unimplemented!();
    }
}

impl Subscribe for Router {
    type Error = Void;
    type Stream = AddrStream;
    fn subscribe(&self, name: &Name) -> AddrStream {
        unimplemented!();
    }
}

impl Future for ResolveHostFuture {
    type Item = Vec<IpAddr>;
    type Error = Error;
    #[inline(always)]
    fn poll(&mut self) -> Result<Async<Vec<IpAddr>>, Error> {
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
    type Item = Vec<IpAddr>;
    type Error = Void;
    #[inline(always)]
    fn poll(&mut self) -> Result<Async<Option<Vec<IpAddr>>>, Void> {
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
