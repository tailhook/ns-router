use std::sync::Arc;

use futures::{Stream, Future, Async};
use tokio_core::reactor::Handle;
use void::{Void, unreachable};

use internal::Table;
use config::Config;


pub struct ResolverFuture<S> {
    stream: S,
    table: Arc<Table>,
    handle: Handle,
}

impl<S> ResolverFuture<S> {
    pub fn new(stream: S, table: &Arc<Table>, handle: &Handle)
        -> ResolverFuture<S>
        where S: Stream<Item=Arc<Config>, Error=Void>
    {
        ResolverFuture {
            stream,
            table: table.clone(),
            handle: handle.clone(),
        }
    }
}

impl<S> Future for ResolverFuture<S>
    where S: Stream<Item=Arc<Config>, Error=Void>
{
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Result<Async<()>, ()> {
        match self.stream.poll() {
            Ok(Async::Ready(Some(c))) => self.table.cfg.put(&c),
            Ok(Async::Ready(None)) => return Ok(Async::Ready(())),
            Ok(Async::NotReady) => {},
            Err(e) => unreachable(e),
        }
        Ok(Async::NotReady)
    }
}
