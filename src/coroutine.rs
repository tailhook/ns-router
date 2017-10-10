use std::sync::{Arc, Weak};
use std::net::IpAddr;
use std::mem;

use abstract_ns::{Address, Name, Error};
use futures::future::Shared;
use futures::stream::FuturesUnordered;
use futures::sync::mpsc::{UnboundedReceiver};
use futures::sync::oneshot;
use futures::{Stream, Future, Async};
use tokio_core::reactor::{Handle, Timeout};
use void::{Void, unreachable};

use config::{Config, Suffix};
use internal::{Table, Request, reply, fail};
use slot;
use subscr::{SubscrFuture, HostNoOpSubscr, NoOpSubscr};


pub struct ResolverFuture {
    update_tx: oneshot::Sender<()>,
    update_rx: Shared<oneshot::Receiver<()>>,
    requests: UnboundedReceiver<Request>,
    futures: FuturesUnordered<Box<Future<Item=FutureResult, Error=Void>>>,
    table: Weak<Table>,
    handle: Handle,
}

pub(crate) trait Continuation {
    fn restart(&mut self, res: &mut ResolverFuture, cfg: &Arc<Config>);
}

pub(crate) enum FutureResult {
    Done,
    Stop,
    UpdateConfig {
        cfg: Arc<Config>,
        next: Box<Future<Item=FutureResult, Error=Void>>,
    },
    Restart {
        task: Box<Continuation>,
    },
    DelayRestart {
        task: Box<Continuation>,
    },
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
    pub fn update_rx(&self) -> Shared<oneshot::Receiver<()>> {
        self.update_rx.clone()
    }
}

pub(crate) fn get_suffix<'x>(cfg: &'x Arc<Config>, name: &str) -> &'x Suffix {
    if let Some(ref suf) = cfg.suffixes.get(name) {
        return suf;
    }
    for (idx, _) in name.match_indices('.') {
        if let Some(suf) = cfg.suffixes.get(&name[idx+1..]) {
            return suf;
        }
    }
    return &cfg.root;
}

impl ResolverFuture {
    pub(crate) fn spawn<F>(&mut self, future: F)
        where F: Future<Item=FutureResult, Error=Void> + 'static,
    {
        self.futures.push(Box::new(future)
            as Box<Future<Item=FutureResult, Error=Void>>)
    }
    fn resolve_host(&mut self, cfg: &Arc<Config>,
        name: Name, tx: oneshot::Sender<Result<Vec<IpAddr>, Error>>)
    {
        // need to retry resolving static host because the config might just
        // arrived right now
        if let Some(value) = cfg.hosts.get(&name) {
            reply(&name, tx, value.clone());
            return;
        }
        if let Some(ref res) = get_suffix(cfg, name.as_ref()).host_resolver {
            res.resolve_host(self, cfg, name, tx);
        } else {
            fail(&name, tx, Error::NameNotFound);
        }
    }
    fn resolve(&mut self, cfg: &Arc<Config>,
        name: Name, tx: oneshot::Sender<Result<Address, Error>>)
    {
        // need to retry resolving static host because the config might just
        // arrived right now
        if let Some(value) = cfg.services.get(&name) {
            reply(&name, tx, value.clone());
            return;
        }
        if let Some(ref res) = get_suffix(cfg, name.as_ref()).resolver {
            res.resolve(self, cfg, name, tx);
        } else {
            fail(&name, tx, Error::NameNotFound);
        }
    }
    pub fn host_subscribe(&mut self, cfg: &Arc<Config>,
        name: Name, tx: slot::Sender<Vec<IpAddr>>)
    {
        if let Some(value) = cfg.hosts.get(&name) {
            let ok = tx.swap(value.clone()).is_ok();
            if ok {
                SubscrFuture::spawn_in(self, HostNoOpSubscr { name, tx });
            }
            return;
        }
        if let Some(ref sub) = get_suffix(cfg, name.as_ref()).host_subscriber {
            sub.host_subscribe(self, sub, cfg, name, tx);
        } else {
            // in subscription functions we don't fail, we just wait
            // for next opportunity (configuration reload?)
            SubscrFuture::spawn_in(self, HostNoOpSubscr { name, tx });
        }
    }
    pub fn subscribe(&mut self, cfg: &Arc<Config>,
        name: Name, tx: slot::Sender<Address>)
    {
        if let Some(value) = cfg.services.get(&name) {
            let ok = tx.swap(value.clone()).is_ok();
            if ok {
                SubscrFuture::spawn_in(self, NoOpSubscr { name, tx });
            }
            return;
        }
        if let Some(ref sub) = get_suffix(cfg, name.as_ref()).subscriber {
            sub.subscribe(self, sub, cfg, name, tx);
        } else {
            // in subscription functions we don't fail, we just wait
            // for next opportunity (configuration reload?)
            SubscrFuture::spawn_in(self, NoOpSubscr { name, tx });
        }
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
        if let Some(mut cfg) = table.cfg.get() {
            loop {
                let inp = self.requests.poll()
                    .map_err(|_| error!("Router input stream is failed"))?;
                match inp {
                    Async::Ready(Some(ResolveHost(n, tx))) => {
                        self.resolve_host(&cfg, n, tx);
                    }
                    Async::Ready(Some(Resolve(n, tx))) => {
                        self.resolve(&cfg, n, tx);
                    }
                    Async::Ready(Some(HostSubscribe(n, tx))) => {
                        self.host_subscribe(&cfg, n, tx);
                    }
                    Async::Ready(Some(Subscribe(n, tx))) => {
                        self.subscribe(&cfg, n, tx);
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
                    UpdateConfig { cfg: new_cfg, next } => {
                        table.cfg.put(&new_cfg);
                        cfg = new_cfg;
                        let (tx, rx) = oneshot::channel();
                        let tx = mem::replace(&mut self.update_tx, tx);
                        self.update_rx = rx.shared();
                        tx.send(()).ok();
                        self.futures.push(next);
                    }
                    Restart { mut task } => {
                        task.restart(self, &cfg);
                    }
                    DelayRestart { task } => {
                        self.futures.push(Box::new(
                            Timeout::new(cfg.restart_delay, &self.handle)
                            .expect("can always set timeout")
                            .map_err(|_| -> Void { unreachable!() })
                            .map(move |_| Restart { task })
                        ) as Box<Future<Item=_, Error=_>>);
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
                    Restart { .. } => unreachable!(),
                    DelayRestart { .. } => unreachable!(),
                }
            }
        }
        Ok(Async::NotReady)
    }
}
