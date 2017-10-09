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


pub struct ResolverFuture {
    update_tx: oneshot::Sender<()>,
    update_rx: Shared<oneshot::Receiver<()>>,
    requests: UnboundedReceiver<Request>,
    futures: FuturesUnordered<Box<Future<Item=FutureResult, Error=Void>>>,
    table: Weak<Table>,
    handle: Handle,
}

pub(crate) enum FutureResult {
  Done,
  Stop,
  UpdateConfig {
      cfg: Arc<Config>,
      next: Box<Future<Item=FutureResult, Error=Void>>,
  },
  ResubscribeHost { name: Name, tx: slot::Sender<Vec<IpAddr>> },
  Resubscribe { name: Name, tx: slot::Sender<Address> },
}

fn mapper<S>(res: Result<(Option<Arc<Config>>, S), (Void, S)>)
    -> Result<FutureResult, Void>
    where S: Stream<Item=Arc<Config>, Error=Void> + 'static
{
    match res {
        Ok((None, _)) => Ok(FutureResult::Stop),
        Ok((Some(cfg), stream)) => Ok(FutureResult::UpdateConfig {
            cfg,
            next: Box::new(stream.into_future().then(mapper)),
        }),
        Err((e, _)) => unreachable(e),
    }
}

impl ResolverFuture {
    pub fn new<S>(config: S, requests: UnboundedReceiver<Request>,
        table: &Arc<Table>, handle: &Handle)
        -> ResolverFuture
        where S: Stream<Item=Arc<Config>, Error=Void> + 'static
    {
        let (tx, rx) = oneshot::channel();
        let mut futures = FuturesUnordered::new();
        futures.push(
            Box::new(config.into_future().then(mapper))
            as Box<Future<Item=FutureResult, Error=Void>>);
        ResolverFuture {
            requests,
            update_tx: tx,
            update_rx: rx.shared(),
            futures: futures,
            table: Arc::downgrade(table),
            handle: handle.clone(),
        }
    }
}

impl ResolverFuture {
    pub(crate) fn spawn<F>(&mut self, future: F)
        where F: Future<Item=FutureResult, Error=Void> + 'static,
    {
        self.futures.push(Box::new(future)
            as Box<Future<Item=FutureResult, Error=Void>>)
    }
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
                res.resolve_host(self, cfg, name, tx);
            } else {
                fail(&name, tx, Error::NameNotFound);
            }
            return
        }
        for (idx, _) in name.as_ref().match_indices('.') {
            if let Some(suf) = cfg.suffixes.get(&name.as_ref()[idx+1..]) {
                if let Some(ref res) = suf.host_resolver {
                    res.resolve_host(self, cfg, name.clone(), tx);
                } else {
                    fail(&name, tx, Error::NameNotFound);

                }
                return
            }
        }
        if let Some(ref res) = cfg.host_resolver {
            res.resolve_host(self, cfg, name, tx);
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
                res.resolve(self, cfg, name, tx);
            } else {
                fail(&name, tx, Error::NameNotFound);
            }
            return
        }
        for (idx, _) in name.as_ref().match_indices('.') {
            if let Some(suf) = cfg.suffixes.get(&name.as_ref()[idx+1..]) {
                if let Some(ref res) = suf.resolver {
                    res.resolve(self, cfg, name.clone(), tx);
                } else {
                    fail(&name, tx, Error::NameNotFound);

                }
                return
            }
        }
        if let Some(ref res) = cfg.resolver {
            res.resolve(self, cfg, name, tx);
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

impl Future for ResolverFuture {
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Result<Async<()>, ()> {
        use internal::Request::*;
        let table = match self.table.upgrade() {
            Some(x) => x,
            None => return Ok(Async::Ready(())),
        };
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
            while let Ok(Async::Ready(Some(state))) = self.futures.poll() {
                use self::FutureResult::*;
                match state {
                    Done => {}
                    Stop => return Ok(Async::Ready(())),
                    UpdateConfig { cfg, next } => {
                        table.cfg.put(&cfg);
                        let (tx, rx) = oneshot::channel();
                        let tx = mem::replace(&mut self.update_tx, tx);
                        self.update_rx = rx.shared();
                        tx.send(()).ok();
                        self.futures.push(next);
                    }
                    ResubscribeHost { name, tx } => {
                        self.host_subscribe(&table, &cfg, name, tx)
                    }
                    Resubscribe { name, tx } => {
                        self.subscribe(&table, &cfg, name, tx)
                    }
                }
            }
        } else {
            while let Ok(Async::Ready(Some(state))) = self.futures.poll() {
                use self::FutureResult::*;
                match state {
                    Done => {}
                    Stop => return Ok(Async::Ready(())),
                    UpdateConfig { cfg, next } => {
                        table.cfg.put(&cfg);
                        let (tx, rx) = oneshot::channel();
                        let tx = mem::replace(&mut self.update_tx, tx);
                        self.update_rx = rx.shared();
                        tx.send(()).ok();
                        self.futures.push(next);
                        // we have a config, so we will not recurse more
                        return self.poll()
                    }
                    ResubscribeHost { .. } => unreachable!(),
                    Resubscribe { .. } => unreachable!(),
                }
            }
        }
        Ok(Async::NotReady)
    }
}
