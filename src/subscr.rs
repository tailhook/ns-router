use std::net::IpAddr;
use std::sync::Arc;

use abstract_ns::{Name, Address};
use futures::{Future, Stream, Async};
use futures::sync::oneshot;
use futures::future::Shared;
use void::Void;

use slot;
use internal_traits::{HostSubscriber, Subscriber};
use config::Config;
use coroutine::{ResolverFuture, FutureResult, Continuation};


pub(crate) struct SubscrFuture<F: Task> {
    pub update_rx: Shared<oneshot::Receiver<()>>,
    pub task: Option<F>,
}

pub(crate) trait Task {
    fn poll(&mut self);
    fn restart(self, res: &mut ResolverFuture, cfg: &Arc<Config>);
}

pub(crate) struct Subscr<S: Stream<Item=Address>> {
    pub name: Name,
    pub subscriber: Arc<Subscriber>,
    pub source: S,
    pub tx: slot::Sender<Address>,
}

pub(crate) struct HostSubscr<S: Stream<Item=Vec<IpAddr>>> {
    pub name: Name,
    pub subscriber: Arc<HostSubscriber>,
    pub source: S,
    pub tx: slot::Sender<Vec<IpAddr>>,
}

pub(crate) struct HostMemSubscr {
    pub name: Name,
    pub tx: slot::Sender<Vec<IpAddr>>,
}

pub(crate) struct MemSubscr {
    pub name: Name,
    pub tx: slot::Sender<Address>,
}

pub(crate) struct Wrapper<T: Task>(Option<T>);

impl<T: Task> Continuation for Wrapper<T> {
    fn restart(&mut self, res: &mut ResolverFuture, cfg: &Arc<Config>) {
        self.0.take().expect("continuation called twice")
            .restart(res, cfg)
    }
}

impl<F: Task + 'static> Future for SubscrFuture<F> {
    type Item = FutureResult;
    type Error = Void;
    fn poll(&mut self) -> Result<Async<FutureResult>, Void> {
        match self.update_rx.poll() {
            Ok(Async::Ready(_)) | Err(_) => {
                return Ok(Async::Ready(FutureResult::Restart {
                    task: Box::new(Wrapper(Some(
                        self.task.take().expect("future polled twice"))))
                        as Box<Continuation>,
                }));

            }
            Ok(Async::NotReady) => {},
        }
        self.task.as_mut().expect("future polled twice").poll();
        Ok(Async::NotReady)
    }
}

impl<S: Stream<Item=Address>> Task for Subscr<S> {
    fn restart(self, res: &mut ResolverFuture, cfg: &Arc<Config>) {
        unimplemented!();
    }
    fn poll(&mut self) {
        unimplemented!();
    }
}

impl<S: Stream<Item=Vec<IpAddr>>> Task for HostSubscr<S> {
    fn restart(self, res: &mut ResolverFuture, cfg: &Arc<Config>) {
        unimplemented!();
    }
    fn poll(&mut self) {
        unimplemented!();
    }
}

impl Task for HostMemSubscr {
    fn restart(self, res: &mut ResolverFuture, cfg: &Arc<Config>) {
        // it's cheap to just resolve it again
        res.host_subscribe(cfg, self.name, self.tx);
    }
    fn poll(&mut self) {
        // do nothing until config changes
    }
}

impl Task for MemSubscr {
    fn restart(self, res: &mut ResolverFuture, cfg: &Arc<Config>) {
        // it's cheap to just resolve it again
        res.subscribe(cfg, self.name, self.tx);
    }
    fn poll(&mut self) {
        // do nothing until config changes
    }
}

impl<T: Task + 'static> SubscrFuture<T> {
    pub fn spawn_in(r: &mut ResolverFuture, task: T) {
        let update_rx = r.update_rx();
        r.spawn(SubscrFuture {
            update_rx,
            task: Some(task),
        });
    }
}
