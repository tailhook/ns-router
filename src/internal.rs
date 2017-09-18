use std::sync::Arc;
use std::net::IpAddr;

use abstract_ns::{Name, Error, Address};
use futures::Stream;
use futures::sync::oneshot;
use tokio_core::reactor::Handle;

use cell::ConfigCell;
use config::Config;
use coroutine::ResolverFuture;
use slot;
use void::Void;


#[derive(Debug)]
pub struct Table {
    pub cfg: ConfigCell,
}

impl Table {
    pub fn new<S>(stream: S, handle: &Handle) -> Arc<Table>
        where S: Stream<Item=Arc<Config>, Error=Void> + 'static
    {
        let table = Arc::new(Table {
            cfg: ConfigCell::new(),
        });
        handle.spawn(ResolverFuture::new(stream, &table, &handle));
        return table;
    }

    pub fn resolve_host(&self, name: &Name,
        tx: oneshot::Sender<Result<Vec<IpAddr>, Error>>)
    {
        // shortcut if config exists and this is an in-memory host
        if let Some(cfg) = self.cfg.get() {
            if let Some(value) = cfg.hosts.get(name) {
                tx.send(Ok(value.clone()))
                    .map_err(|_| {
                        trace!("{:?} resolved into {:?} but dropped",
                            name, value);
                    })
                    .ok();
                return;
            }
        }
        unimplemented!();
    }

    pub fn resolve(&self, name: &Name,
        tx: oneshot::Sender<Result<Address, Error>>)
    {
        // shortcut if config exists and this is an in-memory host
        if let Some(cfg) = self.cfg.get() {
            if let Some(value) = cfg.services.get(name) {
                tx.send(Ok(value.clone()))
                    .map_err(|value| {
                        trace!("{:?} resolved into {:?} but dropped",
                            name, value);
                    })
                    .ok();
                return;
            }
        }
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
