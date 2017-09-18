use std::sync::Arc;
use std::net::IpAddr;

use abstract_ns::{Name, Error, Address};
use futures::Stream;
use futures::sync::oneshot;
use tokio_core::reactor::Handle;

use config::Config;
use coroutine::ResolverFuture;
use slot;
use void::Void;


#[derive(Debug)]
pub struct Table {
}

impl Table {
    pub fn new<S>(stream: S, handle: &Handle) -> Arc<Table>
        where S: Stream<Item=Arc<Config>, Error=Void> + 'static
    {
        let table = Arc::new(Table {
        });
        handle.spawn(ResolverFuture::new(stream, &table, &handle));
        return table;
    }

    pub fn resolve_host(&self, name: &Name,
        tx: oneshot::Sender<Result<Vec<IpAddr>, Error>>)
    {
        unimplemented!();
    }

    pub fn resolve(&self, name: &Name,
        tx: oneshot::Sender<Result<Address, Error>>)
    {
        unimplemented!();
    }

    pub fn subscribe_host(&self, name: &Name,
        tx: slot::Sender<Vec<IpAddr>>)
    {
        unimplemented!();
    }

    pub fn subscribe(&self, name: &Name, tx: slot::Sender<Address>) {
        unimplemented!();
    }
}
