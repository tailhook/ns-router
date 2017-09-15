use std::net::IpAddr;

use abstract_ns::{Name, Address, Error};
use abstract_ns::{Resolve, ResolveHost, Subscribe, HostSubscribe};
use futures::sync::oneshot;
use futures::{Future, Async};

use router::Router;

pub struct ResolveHostFuture(oneshot::Receiver<Result<Vec<IpAddr>, Error>>);
pub struct ResolveFuture(oneshot::Receiver<Result<Address, Error>>);
pub struct HostStream(slot::Receiver<Vec<IpAddr>>);
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

impl Future for ResolveHostFuture {
    type Item = Vec<IpAddr>;
    type Error = Error;
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
    fn poll(&mut self) -> Result<Async<Address>, Error> {
        match self.0.poll().map_err(|e| Error::TemporaryError(e.into()))? {
            Async::NotReady => Ok(Async::NotReady),
            Async::Ready(Ok(r))  => Ok(Async::Ready(r)),
            Async::Ready(Err(e))  => Err(e),
        }
    }
}
