use std::sync::{Arc, Weak};
use std::net::IpAddr;

use abstract_ns::{Address, Name, Error};
use futures::sync::mpsc::{UnboundedReceiver};
use futures::sync::oneshot;
use futures::{Stream, Future, Async};
use tokio_core::reactor::Handle;
use void::{Void, unreachable};

use config::Config;
use internal::{Table, Request};
use slot;


pub struct ResolverFuture<S> {
    config: S,
    requests: UnboundedReceiver<Request>,
    table: Weak<Table>,
    handle: Handle,
}

impl<S> ResolverFuture<S> {
    pub fn new(config: S, requests: UnboundedReceiver<Request>,
        table: &Arc<Table>, handle: &Handle)
        -> ResolverFuture<S>
        where S: Stream<Item=Arc<Config>, Error=Void>
    {
        ResolverFuture {
            config, requests,
            table: Arc::downgrade(table),
            handle: handle.clone(),
        }
    }
}
impl<S> ResolverFuture<S> {
    fn resolve_host(&mut self, table: &Arc<Table>, cfg: &Arc<Config>,
        name: Name, tx: oneshot::Sender<Result<Vec<IpAddr>, Error>>)
    {
        unimplemented!();
    }
    fn resolve(&mut self, table: &Arc<Table>, cfg: &Arc<Config>,
        name: Name, tx: oneshot::Sender<Result<Address, Error>>)
    {
        unimplemented!();
    }
    fn host_subscribe(&mut self, table: &Arc<Table>, cfg: &Arc<Config>,
        name: Name, tx: slot::Sender<Vec<IpAddr>>)
    {
        unimplemented!();
    }
    fn subscribe(&mut self, table: &Arc<Table>, cfg: &Arc<Config>,
        name: Name, tx: slot::Sender<Address>)
    {
        unimplemented!();
    }
}

impl<S> Future for ResolverFuture<S>
    where S: Stream<Item=Arc<Config>, Error=Void>
{
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Result<Async<()>, ()> {
        use internal::Request::*;
        let table = match self.table.upgrade() {
            Some(x) => x,
            None => return Ok(Async::Ready(())),
        };
        match self.config.poll() {
            Ok(Async::Ready(Some(c))) => table.cfg.put(&c),
            Ok(Async::Ready(None)) => return Ok(Async::Ready(())),
            Ok(Async::NotReady) => {},
            Err(e) => unreachable(e),
        }
        if let Some(cfg) = table.cfg.get() {
            loop {
                let inp = self.requests.poll()
                    .map_err(|_| error!("Router input stream is failed"))?;
                match inp {
                    Async::Ready(Some(ResolveHost(n, tx))) => {
                        self.resolve_host(&table, &cfg, n, tx);
                    }
                    Async::Ready(Some(Resolve(n, tx))) => {
                        self.resolve(&table, &cfg, n, tx);
                    }
                    Async::Ready(Some(HostSubscribe(n, tx))) => {
                        self.host_subscribe(&table, &cfg, n, tx);
                    }
                    Async::Ready(Some(Subscribe(n, tx))) => {
                        self.subscribe(&table, &cfg, n, tx);
                    }
                    Async::Ready(None) => {
                        error!("Router input stream is done");
                        return Ok(Async::Ready(()));
                    }
                    Async::NotReady => {
                        break;
                    }
                }
            }
        }
        Ok(Async::NotReady)
    }
}
