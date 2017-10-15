use std::fmt;
use std::sync::Arc;

use abstract_ns::{Name, Error, Address, IpList};
use futures::Stream;
use futures::sync::oneshot;
use futures::sync::mpsc::{UnboundedSender, unbounded};
use tokio_core::reactor::Handle;
use void::Void;

use cell::ConfigCell;
use config::Config;
use coroutine::{ResolverFuture, Continuation};
use multisubscr::MultiSubscr;
use name::InternalName;
use slot;
use subscr::{Wrapper};


#[derive(Debug)]
pub(crate) enum Request {
    ResolveHost(Name, oneshot::Sender<Result<IpList, Error>>),
    ResolveHostPort(Name, u16, oneshot::Sender<Result<Address, Error>>),
    Resolve(Name, oneshot::Sender<Result<Address, Error>>),
    HostSubscribe(Name, slot::Sender<IpList>),
    Subscribe(Name, slot::Sender<Address>),
    Task(Box<Continuation>),
}


#[derive(Debug)]
pub(crate) struct Table {
    pub cfg: ConfigCell,
    pub requests: UnboundedSender<Request>,
}

impl Table {
    pub fn new<S>(stream: S, handle: &Handle) -> Arc<Table>
        where S: Stream<Item=Arc<Config>, Error=Void> + 'static
    {
        let (tx, rx) = unbounded();
        let table = Arc::new(Table {
            cfg: ConfigCell::new(),
            requests: tx,
        });
        handle.spawn(ResolverFuture::new(stream, rx, &table, &handle));
        return table;
    }

    pub fn resolve_host(&self, name: &Name,
        tx: oneshot::Sender<Result<IpList, Error>>)
    {
        // shortcut if config exists and this is an in-memory host
        if let Some(cfg) = self.cfg.get() {
            if let Some(value) = cfg.hosts.get(name) {
                reply(name, tx, value.clone());
                return;
            }
        }
        match self.requests.unbounded_send(
            Request::ResolveHost(name.clone(), tx))
        {
            Ok(()) => {}
            Err(e) => match e.into_inner() {
                Request::ResolveHost(name, tx) => {
                    fail(&name, tx, Error::TemporaryError(
                        "Resolver is down".into()));
                }
                _ => unreachable!(),
            }
        }
    }

    pub fn resolve_host_with_port(&self, name: &Name, port: u16,
        tx: oneshot::Sender<Result<Address, Error>>)
    {
        // shortcut if config exists and this is an in-memory host
        if let Some(cfg) = self.cfg.get() {
            if let Some(value) = cfg.hosts.get(name) {
                reply(name, tx, value.with_port(port));
                return;
            }
        }
        match self.requests.unbounded_send(
            Request::ResolveHostPort(name.clone(), port, tx))
        {
            Ok(()) => {}
            Err(e) => match e.into_inner() {
                Request::ResolveHostPort(name, _, tx) => {
                    fail(&name, tx, Error::TemporaryError(
                        "Resolver is down".into()));
                }
                _ => unreachable!(),
            }
        }
    }

    pub fn resolve(&self, name: &Name,
        tx: oneshot::Sender<Result<Address, Error>>)
    {
        // shortcut if config exists and this is an in-memory host
        if let Some(cfg) = self.cfg.get() {
            if let Some(value) = cfg.services.get(name) {
                reply(name, tx, value.clone());
                return;
            }
        }
        match self.requests.unbounded_send(
            Request::Resolve(name.clone(), tx))
        {
            Ok(()) => {}
            Err(e) => match e.into_inner() {
                Request::Resolve(name, tx) => {
                    fail(&name, tx, Error::TemporaryError(
                        "Resolver is down".into()));
                }
                _ => unreachable!(),
            }
        }
    }

    pub fn subscribe_host(&self, name: &Name, tx: slot::Sender<IpList>) {
        self.requests.unbounded_send(
            Request::HostSubscribe(name.clone(), tx)).ok();
        // TODO(tailhook) log a warning on error?
    }

    pub fn subscribe(&self, name: &Name, tx: slot::Sender<Address>) {
        self.requests.unbounded_send(
            Request::Subscribe(name.clone(), tx)).ok();
        // TODO(tailhook) log a warning on error?
    }

    pub(crate) fn subscribe_stream<S>(&self,
        stream: S, tx: slot::Sender<Address>)
        where S: Stream<Item=Vec<InternalName>> + 'static,
              S::Error: fmt::Display,
    {
        self.requests.unbounded_send(
            Request::Task(Wrapper::wrap(MultiSubscr::new(stream, tx)))).ok();
        // TODO(tailhook) log a warning on error?
    }
}

pub fn reply<X: Send + fmt::Debug + 'static>(name: &Name,
    tx: oneshot::Sender<Result<X, Error>>, value: X)
{
    tx.send(Ok(value))
        .map_err(|value| {
            trace!("{:?} resolved into {:?} but dropped",
                name, value.unwrap());
        })
        .ok();
}

pub fn fail<X: fmt::Debug>(name: &Name,
    tx: oneshot::Sender<Result<X, Error>>, error: Error)
{
    tx.send(Err(error))
        .map_err(|err| {
            trace!("{:?} resolved into, error but dropped: {}",
                name, err.unwrap_err());
        })
        .ok();
}
