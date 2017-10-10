use std::net::IpAddr;

use abstract_ns::{Name, Address};
use futures::{Future, Async};
use futures::sync::oneshot;
use futures::future::Shared;
use void::Void;

use slot;
use coroutine::FutureResult;


pub(crate) struct SubscrFuture<F: Task> {
    pub update_rx: Shared<oneshot::Receiver<()>>,
    pub task: Option<F>,
}

pub(crate) trait Task {
    fn state(self) -> FutureResult;
    fn poll(&mut self);
}

struct Subscr {
}

struct HostSubscr {
}

pub(crate) struct HostMemSubscr {
    pub name: Name,
    pub tx: slot::Sender<Vec<IpAddr>>,
}

pub(crate) struct MemSubscr {
    pub name: Name,
    pub tx: slot::Sender<Address>,
}

impl<F: Task> Future for SubscrFuture<F> {
    type Item = FutureResult;
    type Error = Void;
    fn poll(&mut self) -> Result<Async<FutureResult>, Void> {
        match self.update_rx.poll() {
            Ok(Async::Ready(_)) | Err(_) => {
                return Ok(Async::Ready(self.task
                    .take().expect("future polled twice")
                    .state()));
            }
            Ok(Async::NotReady) => {},
        }
        self.task.as_mut().expect("future polled twice").poll();
        Ok(Async::NotReady)
    }
}

impl Task for Subscr {
    fn state(self) -> FutureResult {
        unimplemented!();
    }
    fn poll(&mut self) {
        unimplemented!();
    }
}

impl Task for HostSubscr {
    fn state(self) -> FutureResult {
        unimplemented!();
    }
    fn poll(&mut self) {
        unimplemented!();
    }
}

impl Task for HostMemSubscr {
    fn state(self) -> FutureResult {
        FutureResult::ResubscribeHost {
            name: self.name,
            tx: self.tx,
        }
    }
    fn poll(&mut self) {
        // do nothing until config changes
    }
}

impl Task for MemSubscr {
    fn state(self) -> FutureResult {
        FutureResult::Resubscribe {
            name: self.name,
            tx: self.tx,
        }
    }
    fn poll(&mut self) {
        // do nothing until config changes
    }
}
