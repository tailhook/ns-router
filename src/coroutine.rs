use std::sync::{Arc, Weak};
use std::net::IpAddr;
use std::mem;

use abstract_ns::{Address, Name, Error};
use futures::future::Shared;
use futures::stream::FuturesUnordered;
use futures::sync::mpsc::{UnboundedReceiver};
use futures::sync::oneshot;
use futures::{Stream, Future, Async};
use tokio_core::reactor::Handle;
use void::{Void, unreachable};

use config::Config;
use internal::{Table, Request, reply, fail};
use slot;


pub struct ResolverFuture<S> {
    config: S,
    update_tx: oneshot::Sender<()>,
    update_rx: Shared<oneshot::Receiver<()>>,
    requests: UnboundedReceiver<Request>,
    subscriptions: FuturesUnordered<SubscrTask>,
    table: Weak<Table>,
    handle: Handle,
}

struct SubscrTask {
    update_rx: Shared<oneshot::Receiver<()>>,
}

enum SubscrState {
}

impl<S> ResolverFuture<S> {
    pub fn new(config: S, requests: UnboundedReceiver<Request>,
        table: &Arc<Table>, handle: &Handle)
        -> ResolverFuture<S>
        where S: Stream<Item=Arc<Config>, Error=Void>
    {
        let (tx, rx) = oneshot::channel();
        ResolverFuture {
            config, requests,
            update_tx: tx,
            update_rx: rx.shared(),
            subscriptions: FuturesUnordered::new(),
            table: Arc::downgrade(table),
            handle: handle.clone(),
        }
    }
}

impl<S> ResolverFuture<S> {
    fn resolve_host(&mut self, table: &Arc<Table>, cfg: &Arc<Config>,
        name: Name, tx: oneshot::Sender<Result<Vec<IpAddr>, Error>>)
    {
        // need to retry resolving static host because the config might just
        // arrived right now
        if let Some(value) = cfg.hosts.get(&name) {
            reply(&name, tx, value.clone());
            return;
        }
        if let Some(ref suf) = cfg.suffixes.get(name.as_ref()) {
            if let Some(ref res) = suf.host_resolver {
                res.resolve_host(cfg, name, tx);
            } else {
                fail(&name, tx, Error::NameNotFound);
            }
            return
        }
        for (idx, _) in name.as_ref().match_indices('.') {
            if let Some(suf) = cfg.suffixes.get(&name.as_ref()[idx+1..]) {
                if let Some(ref res) = suf.host_resolver {
                    res.resolve_host(cfg, name.clone(), tx);
                } else {
                    fail(&name, tx, Error::NameNotFound);

                }
                return
            }
        }
        if let Some(ref res) = cfg.host_resolver {
            res.resolve_host(cfg, name, tx);
        } else {
            fail(&name, tx, Error::NameNotFound);
        }
    }
    fn resolve(&mut self, table: &Arc<Table>, cfg: &Arc<Config>,
        name: Name, tx: oneshot::Sender<Result<Address, Error>>)
    {
        // need to retry resolving static host because the config might just
        // arrived right now
        if let Some(value) = cfg.services.get(&name) {
            reply(&name, tx, value.clone());
            return;
        }
        if let Some(ref suf) = cfg.suffixes.get(name.as_ref()) {
            if let Some(ref res) = suf.resolver {
                res.resolve(cfg, name, tx);
            } else {
                fail(&name, tx, Error::NameNotFound);
            }
            return
        }
        for (idx, _) in name.as_ref().match_indices('.') {
            if let Some(suf) = cfg.suffixes.get(&name.as_ref()[idx+1..]) {
                if let Some(ref res) = suf.resolver {
                    res.resolve(cfg, name.clone(), tx);
                } else {
                    fail(&name, tx, Error::NameNotFound);

                }
                return
            }
        }
        if let Some(ref res) = cfg.resolver {
            res.resolve(cfg, name, tx);
        } else {
            fail(&name, tx, Error::NameNotFound);
        }
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
            Ok(Async::Ready(Some(c))) => {
                table.cfg.put(&c);
                let (tx, rx) = oneshot::channel();
                let tx = mem::replace(&mut self.update_tx, tx);
                self.update_rx = rx.shared();
                tx.send(()).ok();
            },
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

impl Future for SubscrTask {
    type Item = SubscrState;
    type Error = Void;
    fn poll(&mut self) -> Result<Async<SubscrState>, Void> {
        unimplemented!();
    }
}
