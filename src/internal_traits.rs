use std::fmt::{Debug};
use std::sync::Arc;
use std::net::IpAddr;

use abstract_ns::{Address, Name, Error};
use abstract_ns::{ResolveHost, Resolve, HostSubscribe, Subscribe};
use futures::{Future, Async};
use futures::sync::oneshot;
use void::Void;

use config::Config;
use coroutine::{ResolverFuture, FutureResult};
use subscr::{SubscrFuture, HostSubscr, Subscr};
use internal::{reply, fail};
use slot;


pub trait HostResolver: Debug + 'static {
    fn resolve_host(&self, res: &mut ResolverFuture, cfg: &Arc<Config>,
        name: Name, tx: oneshot::Sender<Result<Vec<IpAddr>, Error>>);
}

pub trait Resolver: Debug + 'static {
    fn resolve(&self, res: &mut ResolverFuture, cfg: &Arc<Config>,
        name: Name, tx: oneshot::Sender<Result<Address, Error>>);
}
pub trait HostSubscriber: Debug + 'static {
    fn host_subscribe(&self, res: &mut ResolverFuture,
        sub: &Arc<HostSubscriber>, cfg: &Arc<Config>,
        name: Name, tx: slot::Sender<Vec<IpAddr>>);
}
pub trait Subscriber: Debug + 'static {
    fn subscribe(&self, res: &mut ResolverFuture,
        sub: &Arc<Subscriber>, cfg: &Arc<Config>,
        name: Name, tx: slot::Sender<Address>);
}

struct SendResult<F: Future>(Name, F,
    Option<oneshot::Sender<Result<F::Item, Error>>>);

#[derive(Debug)]
pub struct ResolveHostWrapper<R: ResolveHost> {
    resolver: R,
}

#[derive(Debug)]
pub struct ResolveWrapper<R: Resolve> {
    resolver: R,
}

#[derive(Debug)]
pub struct HostSubscribeWrapper<S: HostSubscribe> {
    subscriber: S,
}

#[derive(Debug)]
pub struct SubscribeWrapper<S: Subscribe> {
    subscriber: S,
}

impl<R: ResolveHost + Debug + 'static> HostResolver for ResolveHostWrapper<R> {
    fn resolve_host(&self, res: &mut ResolverFuture, _cfg: &Arc<Config>,
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

impl<R: Resolve + Debug + 'static> Resolver for ResolveWrapper<R> {
    fn resolve(&self, res: &mut ResolverFuture, _cfg: &Arc<Config>,
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

impl<S: HostSubscribe + Debug + 'static> HostSubscribeWrapper<S> {
    pub fn new(subscriber: S) -> HostSubscribeWrapper<S> {
        HostSubscribeWrapper {
            subscriber,
        }
    }
}

impl<S: Subscribe + Debug + 'static> SubscribeWrapper<S> {
    pub fn new(subscriber: S) -> SubscribeWrapper<S> {
        SubscribeWrapper {
            subscriber,
        }
    }
}

impl<S> Subscriber for SubscribeWrapper<S>
    where S: Subscribe + Debug + 'static,
{
    fn subscribe(&self, res: &mut ResolverFuture,
        sub: &Arc<Subscriber>, _cfg: &Arc<Config>,
        name: Name, tx: slot::Sender<Address>)
    {
        let update_rx = res.update_rx();
        res.spawn(SubscrFuture {
            update_rx,
            task: Some(Subscr {
                subscriber: sub.clone(),
                source: self.subscriber.subscribe(&name),
                name, tx,
            }),
        });
    }
}

impl<S> HostSubscriber for HostSubscribeWrapper<S>
    where S: HostSubscribe + Debug + 'static,
{
    fn host_subscribe(&self, res: &mut ResolverFuture,
        sub: &Arc<HostSubscriber>, _cfg: &Arc<Config>,
        name: Name, tx: slot::Sender<Vec<IpAddr>>)
    {
        let update_rx = res.update_rx();
        res.spawn(SubscrFuture {
            update_rx,
            task: Some(HostSubscr {
                subscriber: sub.clone(),
                source: self.subscriber.subscribe_host(&name),
                name, tx,
            }),
        });
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
