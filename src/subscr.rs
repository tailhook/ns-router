use std::net::IpAddr;
use std::sync::Arc;

use abstract_ns::{Name, Address, Error};
use futures::{Future, Stream, Async};
use futures::sync::oneshot;
use futures::future::Shared;
use void::Void;

use slot;
use internal_traits::{HostSubscriber, Subscriber};
use config::Config;
use coroutine::{ResolverFuture, FutureResult, Continuation, get_suffix};


pub(crate) struct SubscrFuture<F: Task> {
    pub update_rx: Shared<oneshot::Receiver<()>>,
    pub task: Option<F>,
}

#[must_use]
pub(crate) enum TaskResult {
    Continue,
    Stop,
    DelayRestart,
}

pub(crate) trait Task {
    fn poll(&mut self) -> TaskResult;
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

pub(crate) struct HostNoOpSubscr {
    pub name: Name,
    pub tx: slot::Sender<Vec<IpAddr>>,
}

pub(crate) struct NoOpSubscr {
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
        match self.task.as_mut().expect("future polled twice").poll() {
            TaskResult::Continue => {}
            TaskResult::Stop => return Ok(Async::Ready(FutureResult::Done)),
            TaskResult::DelayRestart => {
                return Ok(Async::Ready(FutureResult::DelayRestart {
                    task: Box::new(Wrapper(Some(
                        self.task.take().expect("future polled twice"))))
                        as Box<Continuation>,
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
        let ref nsub = get_suffix(cfg, self.name.as_ref()).subscriber;
        if let Some(ref sub) = *nsub {
            if !Arc::ptr_eq(sub, &self.subscriber) {
                sub.subscribe(res, sub, cfg, self.name, self.tx);
            } else {
                SubscrFuture::spawn_in(res, self)
            }
        } else {
            // in subscription functions we don't fail, we just wait
            // for next opportunity (configuration reload?)
            SubscrFuture::spawn_in(res,
                NoOpSubscr { name: self.name, tx: self.tx });
        }
    }
    fn poll(&mut self) -> TaskResult {
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
            Ok(Async::NotReady) => {}
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

impl<S: Stream<Item=Vec<IpAddr>> + 'static> Task for HostSubscr<S>
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
        let ref nsub = get_suffix(cfg, self.name.as_ref()).host_subscriber;
        if let Some(ref sub) = *nsub {
            if !Arc::ptr_eq(sub, &self.subscriber) {
                sub.host_subscribe(res, sub, cfg, self.name, self.tx);
            } else {
                SubscrFuture::spawn_in(res, self)
            }
        } else {
            // in subscription functions we don't fail, we just wait
            // for next opportunity (configuration reload?)
            SubscrFuture::spawn_in(res,
                HostNoOpSubscr { name: self.name, tx: self.tx });
        }
    }
    fn poll(&mut self) -> TaskResult {
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
            Ok(Async::NotReady) => {}
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
