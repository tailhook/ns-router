//! Futures and streams returned from router
//!
use std::sync::Arc;

use abstract_ns::{IpList, Address, Error};
use futures::sync::oneshot;
use futures::{Future, Async, Stream};
use void::Void;

use async_slot as slot;
use config::Config;

/// A future returned from `Router::resolve_host`
#[derive(Debug)]
pub struct ResolveHostFuture(
    pub(crate) oneshot::Receiver<Result<IpList, Error>>);

/// A future returned from `Router::resolve`
#[derive(Debug)]
pub struct ResolveFuture(pub(crate) oneshot::Receiver<Result<Address, Error>>);

/// A stream returned from `Router::host_subscribe`
#[derive(Debug)]
pub struct HostStream(pub(crate) slot::Receiver<IpList>);

/// A stream returned from `Router::subscribe`
#[derive(Debug)]
pub struct AddrStream(pub(crate) slot::Receiver<Address>);

/// A sink that updates router created using `Router::updating_config`
#[derive(Debug)]
pub struct UpdateSink(pub(crate) slot::Sender<Arc<Config>>);


impl UpdateSink {
    /// Update a config
    ///
    /// Returns `true` if send worked (meaning router is still alive).
    pub fn update(&self, config: &Arc<Config>) -> bool {
        self.0.swap(config.clone()).is_ok()
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
