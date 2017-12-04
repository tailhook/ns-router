use std::fmt;
use std::sync::{Arc};
use std::mem;

use abstract_ns::{Address, IpList, Name, Error};
use async_slot as slot;
use futures::future::Shared;
use futures::stream::{FuturesUnordered, Fuse};
use futures::sync::mpsc::{UnboundedReceiver};
use futures::sync::oneshot;
use futures::{Stream, Future, Async};
use tokio_core::reactor::{Handle, Timeout};
use void::{Void, unreachable};

use config::Config;
use internal_traits::Resolver;
use internal::{Request, reply};
use subscr::{SubscrFuture, HostNoOpSubscr, NoOpSubscr};


pub struct ResolverFuture {
    update_tx: oneshot::Sender<()>,
    update_rx: Shared<oneshot::Receiver<()>>,
    requests: Fuse<UnboundedReceiver<Request>>,
    futures: FuturesUnordered<Box<Future<Item=FutureResult, Error=Void>>>,
    current_config: Option<Arc<Config>>,
    handle: Handle,
}

pub(crate) trait Continuation: fmt::Debug {
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
    pub(crate) fn new<S>(config: S, requests: UnboundedReceiver<Request>,
        handle: &Handle)
        -> ResolverFuture
        where S: Stream<Item=Arc<Config>, Error=Void> + 'static
    {
        let (tx, rx) = oneshot::channel();
        let mut futures = FuturesUnordered::new();
        futures.push(
            Box::new(config.into_future().then(mapper))
            as Box<Future<Item=FutureResult, Error=Void>>);
        ResolverFuture {
            requests: requests.fuse(),
            update_tx: tx,
            update_rx: rx.shared(),
            futures: futures,
            handle: handle.clone(),
            current_config: None,
        }
    }
    pub fn update_rx(&self) -> Shared<oneshot::Receiver<()>> {
        self.update_rx.clone()
    }
    pub fn handle(&self) -> &Handle {
        &self.handle
    }
}

pub(crate) fn get_suffix<'x>(cfg: &'x Arc<Config>, name: &str) -> &'x Arc<Resolver> {
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
        name: Name, tx: oneshot::Sender<Result<IpList, Error>>)
    {
        // need to retry resolving static host because the config might just
        // arrived right now
        if let Some(value) = cfg.hosts.get(&name) {
            reply(&name, tx, value.clone());
            return;
        }
        get_suffix(cfg, name.as_ref()).resolve_host(self, cfg, name, tx);
    }
    fn resolve_host_port(&mut self, cfg: &Arc<Config>,
        name: Name, port: u16, tx: oneshot::Sender<Result<Address, Error>>)
    {
        // need to retry resolving static host because the config might just
        // arrived right now
        if let Some(value) = cfg.hosts.get(&name) {
            reply(&name, tx, value.with_port(port));
            return;
        }
        get_suffix(cfg, name.as_ref())
            .resolve_host_port(self, cfg, name, port, tx);
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
        get_suffix(cfg, name.as_ref()).resolve(self, cfg, name, tx);
    }
    pub fn host_subscribe(&mut self, cfg: &Arc<Config>,
        name: Name, tx: slot::Sender<IpList>)
    {
        if let Some(value) = cfg.hosts.get(&name) {
            let ok = tx.swap(value.clone()).is_ok();
            if ok {
                SubscrFuture::spawn_in(self, HostNoOpSubscr { name, tx });
            }
            return;
        }
        let sub = get_suffix(cfg, name.as_ref());
        sub.host_subscribe(self, sub, cfg, name, tx);
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
        let sub = get_suffix(cfg, name.as_ref());
        sub.subscribe(self, sub, cfg, name, tx);
    }
}

impl Future for ResolverFuture {
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Result<Async<()>, ()> {
        use internal::Request::*;
        if let Some(mut cfg) = self.current_config.clone() {
            loop {
                let inp = self.requests.poll()
                    .map_err(|_| error!("Router input stream is failed"))?;
                match inp {
                    Async::Ready(Some(ResolveHost(n, tx))) => {
                        self.resolve_host(&cfg, n, tx);
                    }
                    Async::Ready(Some(ResolveHostPort(n, p, tx))) => {
                        self.resolve_host_port(&cfg, n, p, tx);
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
                    Async::Ready(Some(Task(mut task))) => {
                        task.restart(self, &cfg);
                    }
                    Async::Ready(None) => {
                        break;
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
                        self.current_config = Some(new_cfg.clone());
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
                        self.current_config = Some(cfg);
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
