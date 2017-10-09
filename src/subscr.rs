use futures::{Future, Async};
use futures::sync::oneshot;
use futures::future::Shared;
use void::Void;

use coroutine::FutureResult;


pub(crate) struct SubscrFuture<F: Task> {
    update_rx: Shared<oneshot::Receiver<()>>,
    task: F,
}

pub(crate) trait Task {
    fn state(&mut self) -> FutureResult;
    fn poll(&mut self);
}

struct Subscr {
}

struct HostSubscr {
}

impl<F: Task> Future for SubscrFuture<F> {
    type Item = FutureResult;
    type Error = Void;
    fn poll(&mut self) -> Result<Async<FutureResult>, Void> {
        match self.update_rx.poll() {
            Ok(Async::Ready(_)) | Err(_) => {
                return Ok(Async::Ready(self.task.state()));
            }
            Ok(Async::NotReady) => {},
        }
        self.task.poll();
        Ok(Async::NotReady)
    }
}

impl Task for Subscr {
    fn state(&mut self) -> FutureResult {
        unimplemented!();
    }
    fn poll(&mut self) {
        unimplemented!();
    }
}

impl Task for HostSubscr {
    fn state(&mut self) -> FutureResult {
        unimplemented!();
    }
    fn poll(&mut self) {
        unimplemented!();
    }
}
