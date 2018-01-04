use std::fmt::{Debug};
use std::sync::Arc;

use abstract_ns::{Address, IpList, Name, Error};
use abstract_ns::{HostResolve, Resolve, HostSubscribe, Subscribe};
use async_slot as slot;
use futures::{Future, Async};
use futures::sync::oneshot;
use void::Void;

use config::Config;
use coroutine::{ResolverFuture, FutureResult};
use fuse::Fuse;
use subscr::{SubscrFuture, HostSubscr, Subscr, NoOpSubscr, HostNoOpSubscr};
use internal::{reply, fail};


pub trait Resolver: Debug + 'static {
    fn resolve_host(&self, res: &mut ResolverFuture, cfg: &Arc<Config>,
        name: Name, tx: oneshot::Sender<Result<IpList, Error>>);
    fn resolve_host_port(&self, res: &mut ResolverFuture, cfg: &Arc<Config>,
        name: Name, port: u16, tx: oneshot::Sender<Result<Address, Error>>);
    fn resolve(&self, res: &mut ResolverFuture, cfg: &Arc<Config>,
        name: Name, tx: oneshot::Sender<Result<Address, Error>>);
    fn host_subscribe(&self, res: &mut ResolverFuture,
        sub: &Arc<Resolver>, cfg: &Arc<Config>,
        name: Name, tx: slot::Sender<IpList>);
    fn subscribe(&self, res: &mut ResolverFuture,
        sub: &Arc<Resolver>, cfg: &Arc<Config>,
        name: Name, tx: slot::Sender<Address>);
}

#[must_use = "futures do nothing unless polled"]
struct SendResult<F: Future>(Name, F,
    Option<oneshot::Sender<Result<F::Item, Error>>>);

#[derive(Debug)]
pub struct Wrapper<R> {
    resolver: R,
}

#[derive(Debug)]
pub struct NullResolver;


impl<R:Debug + 'static> Wrapper<R>
    where R: Resolve + HostResolve + Subscribe + HostSubscribe
{
    pub fn new(resolver: R) -> Wrapper<R> {
        Wrapper {
            resolver,
        }
    }
}

impl<R:Debug + 'static> Resolver for Wrapper<R>
    where R: Resolve + HostResolve + Subscribe + HostSubscribe
{
    fn resolve_host(&self, res: &mut ResolverFuture, _cfg: &Arc<Config>,
        name: Name, tx: oneshot::Sender<Result<IpList, Error>>)
    {
        let future = self.resolver.resolve_host(&name);
        res.spawn(SendResult(name, future, Some(tx)));
    }
    fn resolve_host_port(&self, res: &mut ResolverFuture, _cfg: &Arc<Config>,
        name: Name, port: u16, tx: oneshot::Sender<Result<Address, Error>>)
    {
        let future = self.resolver.resolve_host(&name);
        let future = future.map(move |x| x.with_port(port));
        res.spawn(SendResult(name, future, Some(tx)));
    }

    fn resolve(&self, res: &mut ResolverFuture, _cfg: &Arc<Config>,
        name: Name, tx: oneshot::Sender<Result<Address, Error>>)
    {
        let f = self.resolver.resolve(&name);
        res.spawn(SendResult(name, f, Some(tx)));
    }

    fn subscribe(&self, res: &mut ResolverFuture,
        sub: &Arc<Resolver>, _cfg: &Arc<Config>,
        name: Name, tx: slot::Sender<Address>)
    {
        let update_rx = res.update_rx();
        res.spawn(SubscrFuture {
            update_rx,
            task: Some(Subscr {
                subscriber: sub.clone(),
                source: Fuse::new(self.resolver.subscribe(&name)),
                name, tx,
            }),
        });
    }

    fn host_subscribe(&self, res: &mut ResolverFuture,
        sub: &Arc<Resolver>, _cfg: &Arc<Config>,
        name: Name, tx: slot::Sender<IpList>)
    {
        let update_rx = res.update_rx();
        res.spawn(SubscrFuture {
            update_rx,
            task: Some(HostSubscr {
                subscriber: sub.clone(),
                source: Fuse::new(self.resolver.subscribe_host(&name)),
                name, tx,
            }),
        });
    }
}

impl Resolver for NullResolver {
    fn resolve_host(&self, _res: &mut ResolverFuture, _cfg: &Arc<Config>,
        _name: Name, tx: oneshot::Sender<Result<IpList, Error>>)
    {
        tx.send(Err(Error::NameNotFound)).ok();
    }
    fn resolve_host_port(&self, _res: &mut ResolverFuture, _cfg: &Arc<Config>,
        _name: Name, _port: u16, tx: oneshot::Sender<Result<Address, Error>>)
    {
        tx.send(Err(Error::NameNotFound)).ok();
    }

    fn resolve(&self, _res: &mut ResolverFuture, _cfg: &Arc<Config>,
        _name: Name, tx: oneshot::Sender<Result<Address, Error>>)
    {
        tx.send(Err(Error::NameNotFound)).ok();
    }

    fn subscribe(&self, res: &mut ResolverFuture,
        _sub: &Arc<Resolver>, _cfg: &Arc<Config>,
        name: Name, tx: slot::Sender<Address>)
    {
        SubscrFuture::spawn_in(res, NoOpSubscr { name, tx });
    }

    fn host_subscribe(&self, res: &mut ResolverFuture,
        _sub: &Arc<Resolver>, _cfg: &Arc<Config>,
        name: Name, tx: slot::Sender<IpList>)
    {
        SubscrFuture::spawn_in(res, HostNoOpSubscr { name, tx });
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
