use std::fmt::{self, Debug};
use std::sync::Arc;
use std::sync::Mutex;
use std::net::IpAddr;

use abstract_ns::{Address, Name, Error};
use abstract_ns::{ResolveHost, Resolve, HostSubscribe, Subscribe};
use futures::{Future, Async};
use futures::sync::oneshot;
use futures::stream::FuturesUnordered;
use internal::{reply, fail};

use config::Config;
use slot;


pub trait HostResolver: Debug + 'static {
    fn resolve_host(&self, cfg: &Arc<Config>,
        name: Name, tx: oneshot::Sender<Result<Vec<IpAddr>, Error>>);
}

pub trait Resolver: Debug + 'static {
    fn resolve(&self, cfg: &Arc<Config>,
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

pub struct ResolveHostWrapper<R: ResolveHost> {
    resolver: R,
    futures: Mutex<FuturesUnordered<SendResult<R::FutureHost>>>,
}

impl<R: ResolveHost + Debug + 'static> HostResolver for ResolveHostWrapper<R> {
    fn resolve_host(&self, cfg: &Arc<Config>,
        name: Name, tx: oneshot::Sender<Result<Vec<IpAddr>, Error>>)
    {
        let mut future = self.resolver.resolve_host(&name);
        match future.poll() {
            Ok(Async::Ready(v)) => reply(&name, tx, v),
            Ok(Async::NotReady) => {
                self.futures.lock().expect("futures not poisoned")
                    .push(SendResult(name, future, Some(tx)));
            }
            Err(e) => {
                tx.send(Err(e)).ok();
            }
        }
    }
}

impl<R: ResolveHost + Debug + 'static> ResolveHostWrapper<R> {
    pub fn new(resolver: R) -> ResolveHostWrapper<R> {
        ResolveHostWrapper {
            resolver,
            futures: Mutex::new(FuturesUnordered::new()),
        }
    }
}

impl<R: ResolveHost + Debug + 'static> fmt::Debug for ResolveHostWrapper<R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ResolveHostWrapper")
        .field("resolver", &self.resolver)
        .field("futures", &self.futures.lock()
            .map(|x| x.len())
            .unwrap_or_else(|x| x.get_ref().len()))
        .finish()
    }
}

pub struct ResolveWrapper<R: Resolve> {
    resolver: R,
    futures: Mutex<FuturesUnordered<SendResult<R::Future>>>,
}

impl<R: Resolve + Debug + 'static> Resolver for ResolveWrapper<R> {
    fn resolve(&self, cfg: &Arc<Config>,
        name: Name, tx: oneshot::Sender<Result<Address, Error>>)
    {
        let mut future = self.resolver.resolve(&name);
        match future.poll() {
            Ok(Async::Ready(v)) => reply(&name, tx, v),
            Ok(Async::NotReady) => {
                self.futures.lock().expect("futures not poisoned")
                    .push(SendResult(name, future, Some(tx)));
            }
            Err(e) => {
                tx.send(Err(e)).ok();
            }
        }
    }
}

impl<R: Resolve + Debug + 'static> ResolveWrapper<R> {
    pub fn new(resolver: R) -> ResolveWrapper<R> {
        ResolveWrapper {
            resolver,
            futures: Mutex::new(FuturesUnordered::new()),
        }
    }
}

impl<R: Resolve + Debug + 'static> fmt::Debug for ResolveWrapper<R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ResolveWrapper")
        .field("resolver", &self.resolver)
        .field("futures", &self.futures.lock()
            .map(|x| x.len())
            .unwrap_or_else(|x| x.get_ref().len()))
        .finish()
    }
}

impl<F: Future> Future for SendResult<F>
    where F::Item: Send + Debug + 'static,
        F::Error: Into<Error>,
{
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Result<Async<()>, ()> {
        match self.1.poll() {
            Ok(Async::Ready(x)) => {
                let tx = self.2.take().expect("future poled twice");
                reply(&self.0, tx, x);
                Ok(Async::Ready(()))
            }
            Err(e) => {
                let tx = self.2.take().expect("future poled twice");
                fail(&self.0, tx, e.into());
                Ok(Async::Ready(()))
            }
            Ok(Async::NotReady) => Ok(Async::NotReady),
        }
    }
}
