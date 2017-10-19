//! An extension trait that turns resolvers into subscribers
use std::fmt;
use std::time::Duration;
use std::rc::Rc;

use abstract_ns::{Resolve, HostResolve, Subscribe, HostSubscribe, Name};
use abstract_ns::{Address, IpList};
use futures::{Future, Stream, Async};
use tokio_core::reactor::{Handle, Timeout};


/// A subscriber which polls resolver at a regular interval
///
/// Create the instance with `SubscribeExt::interval_subscriber`
#[derive(Debug)]
pub struct IntervalSubscriber<R>(Rc<Internal<R>>);

#[derive(Debug)]
struct Internal<R> {
    resolver: R,
    interval: Duration,
    handle: Handle,
}

enum State<F> {
    Sleeping(Timeout),
    Waiting(F),
}

/// A stream returned by IntervalSubscriber::subscribe
pub struct IntervalResolver<R: Resolve> {
    internal: Rc<Internal<R>>,
    name: Name,
    last_value: Option<Address>,
    state: State<R::Future>,
}

/// A stream returned by IntervalSubscriber::subscribe_host
pub struct IntervalHostResolver<R: HostResolve> {
    internal: Rc<Internal<R>>,
    name: Name,
    last_value: Option<IpList>,
    state: State<R::HostFuture>,
}

/// An extension trait for resolver
///
pub trait SubscribeExt {
    /// Return a subscriber that uses `resolve` or `resolve_host` at a regular
    /// interval
    fn interval_subscriber(self, interval: Duration, handle: &Handle)
        -> IntervalSubscriber<Self>
        where Self: Sized;
}

impl<T: Resolve + HostResolve> SubscribeExt for T {
    fn interval_subscriber(self, interval: Duration, handle: &Handle)
        -> IntervalSubscriber<Self>
        where Self: Sized
    {
        IntervalSubscriber(Rc::new(Internal {
            resolver: self,
            interval,
            handle: handle.clone(),
        }))
    }
}

impl<T: Resolve> Resolve for IntervalSubscriber<T> {
    type Future = T::Future;
    fn resolve(&self, name: &Name) -> Self::Future {
        self.0.resolver.resolve(name)
    }
}

impl<T: HostResolve> HostResolve for IntervalSubscriber<T> {
    type HostFuture = T::HostFuture;
    fn resolve_host(&self, name: &Name) -> Self::HostFuture {
        self.0.resolver.resolve_host(name)
    }
}

impl<T: Resolve> Subscribe for IntervalSubscriber<T> {
    type Error = <T::Future as Future>::Error;
    type Stream = IntervalResolver<T>;
    fn subscribe(&self, name: &Name) -> Self::Stream {
        IntervalResolver {
            internal: self.0.clone(),
            name: name.clone(),
            last_value: None,
            state: State::Waiting(self.resolve(name)),
        }
    }
}

impl<T: HostResolve> HostSubscribe for IntervalSubscriber<T> {
    type HostError = <T::HostFuture as Future>::Error;
    type HostStream = IntervalHostResolver<T>;
    fn subscribe_host(&self, name: &Name) -> Self::HostStream {
        IntervalHostResolver {
            internal: self.0.clone(),
            name: name.clone(),
            last_value: None,
            state: State::Waiting(self.0.resolver.resolve_host(name)),
        }
    }
}


impl<R: HostResolve> Stream for IntervalHostResolver<R> {
    type Item = IpList;
    type Error = <R::HostFuture as Future>::Error;
    fn poll(&mut self) -> Result<Async<Option<IpList>>, Self::Error> {
        use self::State::*;
        loop {
            let mut updated = false;
            match self.state {
                Sleeping(ref mut timer) => {
                    match timer.poll().expect("timer never fails") {
                        Async::NotReady => return Ok(Async::NotReady),
                        Async::Ready(()) => {}
                    }
                }
                Waiting(ref mut future) => {
                    match future.poll()? {
                        Async::NotReady => return Ok(Async::NotReady),
                        Async::Ready(a) => {
                            if self.last_value.as_ref() != Some(&a) {
                                self.last_value = Some(a);
                                updated = true;
                            }
                        }
                    }
                }
            }
            match &mut self.state {
                state @ &mut Sleeping(..) => {
                    *state = Waiting(self.internal.resolver
                        .resolve_host(&self.name));
                }
                state @ &mut Waiting(..) => {
                    *state = Sleeping(Timeout::new(
                        self.internal.interval, &self.internal.handle)
                        .expect("timeout never fails"));
                }
            }
            if updated {
                return Ok(Async::Ready(self.last_value.clone()));
            }
        }
    }
}

impl<R: Resolve> Stream for IntervalResolver<R> {
    type Item = Address;
    type Error = <R::Future as Future>::Error;
    fn poll(&mut self) -> Result<Async<Option<Address>>, Self::Error> {
        use self::State::*;
        loop {
            let mut updated = false;
            match self.state {
                Sleeping(ref mut timer) => {
                    match timer.poll().expect("timer never fails") {
                        Async::NotReady => return Ok(Async::NotReady),
                        Async::Ready(()) => {}
                    }
                }
                Waiting(ref mut future) => {
                    match future.poll()? {
                        Async::NotReady => return Ok(Async::NotReady),
                        Async::Ready(a) => {
                            if self.last_value.as_ref() != Some(&a) {
                                self.last_value = Some(a);
                                updated = true;
                            }
                        }
                    }
                }
            }
            match &mut self.state {
                state @ &mut Sleeping(..) => {
                    *state = Waiting(self.internal.resolver
                        .resolve(&self.name));
                }
                state @ &mut Waiting(..) => {
                    *state = Sleeping(Timeout::new(
                        self.internal.interval, &self.internal.handle)
                        .expect("timeout never fails"));
                }
            }
            if updated {
                return Ok(Async::Ready(self.last_value.clone()));
            }
        }
    }
}

impl<R: Resolve> fmt::Debug for IntervalResolver<R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("InternalResolver")
        .field("last_value", &self.last_value)
        .finish()
    }
}

impl<R: HostResolve> fmt::Debug for IntervalHostResolver<R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("InternalHostResolver")
        .field("last_value", &self.last_value)
        .finish()
    }
}
