use std::fmt::{self, Debug};
use std::sync::Arc;
use std::sync::Mutex;
use std::net::IpAddr;

use abstract_ns::{Address, Name, Error};
use abstract_ns::{ResolveHost, Resolve, HostSubscribe, Subscribe};
use futures::{Future, Async};
use futures::sync::oneshot;
use futures::stream::FuturesUnordered;
use internal::{Table, reply, fail};
use coroutine::{ResolverFuture, FutureResult};

use config::Config;
use slot;
use void::Void;


pub trait HostResolver: Debug + 'static {
    fn resolve_host(&self, res: &mut ResolverFuture, cfg: &Arc<Config>,
        name: Name, tx: oneshot::Sender<Result<Vec<IpAddr>, Error>>);
}

pub trait Resolver: Debug + 'static {
    fn resolve(&self, res: &mut ResolverFuture, cfg: &Arc<Config>,
        name: Name, tx: oneshot::Sender<Result<Address, Error>>);
}
pub trait HostSubscriber: Debug + 'static {
    fn host_subscribe(&self, cfg: &Arc<Config>,
        name: Name, tx: slot::Sender<Vec<IpAddr>>);
}
pub trait Subscriber: Debug + 'static {
    fn subscribe(&self, cfg: &Arc<Config>,
        name: Name, tx: slot::Sender<Address>);
}

struct SendResult<F: Future>(Name, F,
    Option<oneshot::Sender<Result<F::Item, Error>>>);

#[derive(Debug)]
pub struct ResolveHostWrapper<R: ResolveHost> {
    resolver: R,
}

impl<R: ResolveHost + Debug + 'static> HostResolver for ResolveHostWrapper<R> {
    fn resolve_host(&self, res: &mut ResolverFuture, cfg: &Arc<Config>,
        name: Name, tx: oneshot::Sender<Result<Vec<IpAddr>, Error>>)
    {
        let future = self.resolver.resolve_host(&name);
        res.spawn(SendResult(name, future, Some(tx)));
    }
}

impl<R: ResolveHost + Debug + 'static> ResolveHostWrapper<R> {
    pub fn new(resolver: R) -> ResolveHostWrapper<R> {
        ResolveHostWrapper {
            resolver,
        }
    }
}

#[derive(Debug)]
pub struct ResolveWrapper<R: Resolve> {
    resolver: R,
}

impl<R: Resolve + Debug + 'static> Resolver for ResolveWrapper<R> {
    fn resolve(&self, res: &mut ResolverFuture, cfg: &Arc<Config>,
        name: Name, tx: oneshot::Sender<Result<Address, Error>>)
    {
        let f = self.resolver.resolve(&name);
        res.spawn(SendResult(name, f, Some(tx)));
    }
}

impl<R: Resolve + Debug + 'static> ResolveWrapper<R> {
    pub fn new(resolver: R) -> ResolveWrapper<R> {
        ResolveWrapper {
            resolver,
        }
    }
}

impl<F: Future> Future for SendResult<F>
    where F::Item: Send + Debug + 'static,
        F::Error: Into<Error>,
{
    type Item = FutureResult;
    type Error = Void;
    fn poll(&mut self) -> Result<Async<FutureResult>, Void> {
        match self.1.poll() {
            Ok(Async::Ready(x)) => {
                let tx = self.2.take().expect("future poled twice");
                reply(&self.0, tx, x);
                Ok(Async::Ready(FutureResult::Done))
            }
            Err(e) => {
                let tx = self.2.take().expect("future poled twice");
                fail(&self.0, tx, e.into());
                Ok(Async::Ready(FutureResult::Done))
            }
            Ok(Async::NotReady) => Ok(Async::NotReady),
        }
    }
}
