use std::sync::Arc;
use std::net::IpAddr;

use futures::Stream;
use futures::sync::oneshot;
use abstract_ns::{Name, Error, Address};

use slot;
use config::Config;
use void::Void;


#[derive(Debug)]
pub struct Table {
}

impl Table {
    pub fn new<S>(stream: S) -> Table
        where S: Stream<Item=Arc<Config>, Error=Void>
    {
        unimplemented!();
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
