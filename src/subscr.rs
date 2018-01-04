use std::fmt;
use std::sync::Arc;

use abstract_ns::{Name, Address, IpList, Error};
use async_slot as slot;
use futures::{Future, Stream, Async};
use futures::sync::oneshot;
use futures::future::Shared;
use void::Void;

use fuse::Fuse;
use internal_traits::Resolver;
use config::Config;
use coroutine::{ResolverFuture, FutureResult, Continuation, get_suffix};


#[must_use = "futures do nothing unless polled"]
pub(crate) struct SubscrFuture<F: Task> {
    pub update_rx: Shared<oneshot::Receiver<()>>,
    pub task: Option<F>,
}

#[must_use]
pub(crate) enum TaskResult {
    Continue,
    Stop,
    Restart,
    DelayRestart,
}

pub(crate) trait Task {
    fn poll(&mut self) -> TaskResult;
    fn restart(self, res: &mut ResolverFuture, cfg: &Arc<Config>);
}

pub(crate) struct Subscr<S: Stream<Item=Address>> {
    pub name: Name,
    pub subscriber: Arc<Resolver>,
    pub source: Fuse<S>,
    pub tx: slot::Sender<Address>,
}

pub(crate) struct HostSubscr<S: Stream<Item=IpList>> {
    pub name: Name,
    pub subscriber: Arc<Resolver>,
    pub source: Fuse<S>,
    pub tx: slot::Sender<IpList>,
}

pub(crate) struct HostNoOpSubscr {
    pub name: Name,
    pub tx: slot::Sender<IpList>,
}

pub(crate) struct NoOpSubscr {
    pub name: Name,
    pub tx: slot::Sender<Address>,
}

pub(crate) struct Wrapper<T: Task>(Option<T>);

impl<T: Task> fmt::Debug for Wrapper<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Wrapper")
    }
}

impl<T: Task + Send + 'static> Wrapper<T> {
    pub(crate) fn wrap_send(task: T) -> Box<Continuation + Send> {
        Box::new(Wrapper(Some(task))) as Box<Continuation + Send>
    }
}

impl<T: Task + 'static> Wrapper<T> {
    pub(crate) fn wrap(task: T) -> Box<Continuation> {
        Box::new(Wrapper(Some(task))) as Box<Continuation>
    }
}


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
                    task: Wrapper::wrap(
                        self.task.take().expect("future polled twice")),
                }));

            }
            Ok(Async::NotReady) => {},
        }
        match self.task.as_mut().expect("future polled twice").poll() {
            TaskResult::Continue => {}
            TaskResult::Stop => return Ok(Async::Ready(FutureResult::Done)),
            TaskResult::Restart => {
                return Ok(Async::Ready(FutureResult::Restart {
                    task: Wrapper::wrap(
                        self.task.take().expect("future polled twice")),
                }));
            }
            TaskResult::DelayRestart => {
                return Ok(Async::Ready(FutureResult::DelayRestart {
                    task: Wrapper::wrap(
                        self.task.take().expect("future polled twice")),
                }));
            }
        }
        Ok(Async::NotReady)
    }
}

impl<S: Stream<Item=Address> + 'static> Task for Subscr<S>
    where S::Error: Into<Error>,
{
    fn restart(self, res: &mut ResolverFuture, cfg: &Arc<Config>) {
        if let Some(value) =  cfg.services.get(&self.name) {
            let ok = self.tx.swap(value.clone()).is_ok();
            if ok {
                SubscrFuture::spawn_in(res,
                    NoOpSubscr { name: self.name, tx: self.tx });
            }
            return;
        }
        let nsub = get_suffix(cfg, self.name.as_ref());
        if !Arc::ptr_eq(nsub, &self.subscriber) || self.source.is_done() {
            nsub.subscribe(res, nsub, cfg, self.name, self.tx);
        } else {
            SubscrFuture::spawn_in(res, self)
        }
    }
    fn poll(&mut self) -> TaskResult {
        loop {
            match self.source.poll() {
                Ok(Async::Ready(Some(x))) => {
                    if self.tx.swap(x).is_err() {
                        return TaskResult::Stop;
                    }
                }
                Ok(Async::Ready(None))  => {
                    error!("End of stream while following {:?}", self.name);
                    return TaskResult::DelayRestart;
                }
                Err(e) => {
                    error!("Error while following {:?}: {}", self.name,
                        Into::<Error>::into(e));
                    return TaskResult::DelayRestart;
                }
                Ok(Async::NotReady) => break,
            }
        }
        match self.tx.poll_cancel() {
            Ok(Async::NotReady) => {}
            _ => {
                return TaskResult::Stop;
            }
        }
        TaskResult::Continue
    }
}

impl<S: Stream<Item=IpList> + 'static> Task for HostSubscr<S>
    where S::Error: Into<Error>,
{
    fn restart(self, res: &mut ResolverFuture, cfg: &Arc<Config>) {
        if let Some(value) =  cfg.hosts.get(&self.name) {
            let ok = self.tx.swap(value.clone()).is_ok();
            if ok {
                SubscrFuture::spawn_in(res,
                    HostNoOpSubscr { name: self.name, tx: self.tx });
            }
            return;
        }
        let ref nsub = get_suffix(cfg, self.name.as_ref());
        if !Arc::ptr_eq(nsub, &self.subscriber) || self.source.is_done() {
            nsub.host_subscribe(res, nsub, cfg, self.name, self.tx);
        } else {
            SubscrFuture::spawn_in(res, self)
        }
    }
    fn poll(&mut self) -> TaskResult {
        loop {
            match self.source.poll() {
                Ok(Async::Ready(Some(x))) => {
                    if self.tx.swap(x).is_err() {
                        return TaskResult::Stop;
                    }
                }
                Ok(Async::Ready(None))  => {
                    error!("End of stream while following {:?}", self.name);
                    return TaskResult::DelayRestart;
                }
                Err(e) => {
                    error!("Error while following {:?}: {}", self.name,
                        Into::<Error>::into(e));
                    return TaskResult::DelayRestart;
                }
                Ok(Async::NotReady) => break,
            }
        }
        match self.tx.poll_cancel() {
            Ok(Async::NotReady) => {}
            _ => {
                return TaskResult::Stop;
            }
        }
        TaskResult::Continue
    }
}

impl Task for HostNoOpSubscr {
    fn restart(self, res: &mut ResolverFuture, cfg: &Arc<Config>) {
        // it's cheap to just resolve it again
        res.host_subscribe(cfg, self.name, self.tx);
    }
    fn poll(&mut self) -> TaskResult {
        // do nothing until config changes
        TaskResult::Continue
    }
}

impl Task for NoOpSubscr {
    fn restart(self, res: &mut ResolverFuture, cfg: &Arc<Config>) {
        // it's cheap to just resolve it again
        res.subscribe(cfg, self.name, self.tx);
    }
    fn poll(&mut self) -> TaskResult {
        // do nothing until config changes
        TaskResult::Continue
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
