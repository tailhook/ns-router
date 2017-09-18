use std::sync::Arc;

use futures::{Stream, Future, Async};
use tokio_core::reactor::Handle;
use void::Void;

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
        unimplemented!();
    }
}
