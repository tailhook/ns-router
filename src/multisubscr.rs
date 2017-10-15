use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::mem;
use std::sync::Arc;

use abstract_ns::{IpList, Address};
use abstract_ns::addr::union;
use futures::{Stream, Future, Async};
use tokio_core::reactor::Timeout;
use void::{unreachable};

use config::Config;
use coroutine::{ResolverFuture, get_suffix};
use name::InternalName;
use slot;
use subscr::{Task, TaskResult, SubscrFuture};


pub enum State {
    StaticHost(IpList, u16),
    StaticAddr(Address),
    Host(slot::Receiver<IpList>, Option<IpList>, u16),
    Addr(slot::Receiver<Address>, Option<Address>),
}

pub(crate) struct MultiSubscr<S: Stream<Item=Vec<InternalName>>> {
    input: S,
    current: Vec<InternalName>,
    items: HashMap<InternalName, State>,
    timer: Option<Timeout>,
    tx: slot::Sender<Address>,
}

impl State {
    fn addr(&self) -> Option<Cow<Address>> {
        use self::State::*;
        use std::borrow::Cow::*;
        match *self {
            StaticHost(ref list, port) => Some(Owned(list.with_port(port))),
            StaticAddr(ref addr) => Some(Borrowed(addr)),
            Host(_, Some(ref list), port) => Some(Owned(list.with_port(port))),
            Host(_, None, _) => None,
            Addr(_, Some(ref addr)) => Some(Borrowed(addr)),
            Addr(_, None) => None,
        }
    }
    fn is_static(&self) -> bool {
        use self::State::*;
        match *self {
            StaticHost(_, _) => true,
            StaticAddr(_) => true,
            Host(_, _, _) => false,
            Addr(_, _) => false,
        }
    }
    fn is_complete(&self) -> bool {
        use self::State::*;
        match *self {
            StaticHost(_, _) => true,
            StaticAddr(_) => true,
            Host(_, Some(_), _) => true,
            Host(_, None, _) => false,
            Addr(_, Some(_)) => true,
            Addr(_, None) => false,
        }
    }
}

impl<S: Stream<Item=Vec<InternalName>>> MultiSubscr<S> {
    pub(crate) fn new(input: S, tx: slot::Sender<Address>) -> MultiSubscr<S> {
        MultiSubscr {
            tx, input,
            current: Vec::new(),
            items: HashMap::new(),
            timer: None,
        }
    }
    fn send_current(&mut self) -> bool {
        self.tx.swap(union(self.items.values()
            .filter_map(|x| x.addr()))).is_ok()
    }
}

impl<S: Stream<Item=Vec<InternalName>> + 'static> Task for MultiSubscr<S>
    where S::Error: fmt::Display,
{
    fn restart(mut self, res: &mut ResolverFuture, cfg: &Arc<Config>) {
        use self::State::*;
        let mut timeo = Timeout::new(cfg.convergence_delay, res.handle())
            .expect("timeout never fails");
        let mut old_items = mem::replace(&mut self.items, HashMap::new());
        let mut all_ok = true;
        for name in &self.current {
            if let Some(item) = old_items.remove(name) {
                if !item.is_static() {
                    if !item.is_complete() {
                        all_ok = false;
                    }
                    self.items.insert(name.clone(), item);
                    // don't need to check non-static, they're checked by
                    // their own futures
                    continue;
                } // always recheck static, it's cheap
            }
            match *name {
                InternalName::HostPort(ref host, port) => {
                    if let Some(value) = cfg.hosts.get(&host) {
                        self.items.insert(name.clone(),
                            StaticHost(value.clone(), port));
                    } else if let Some(ref sub) =
                        get_suffix(cfg, host.as_ref()).host_subscriber
                    {
                        let (tx, rx) = slot::channel();
                        sub.host_subscribe(res, sub, cfg, host.clone(), tx);
                        self.items.insert(name.clone(),
                            Host(rx, None, port));
                    }
                }
                InternalName::Service(ref service) => {
                    if let Some(value) = cfg.services.get(&service) {
                        self.items.insert(name.clone(),
                                          StaticAddr(value.clone()));
                    } else if let Some(ref sub) =
                        get_suffix(cfg, service.as_ref()).subscriber
                    {
                        let (tx, rx) = slot::channel();
                        sub.subscribe(res, sub, cfg, service.clone(), tx);
                        self.items.insert(name.clone(), Addr(rx, None));
                    }

                }
            }
        }
        if all_ok && self.current.len() > 0 {
            if !self.send_current() {
                return;
            }
        } else {
            match timeo.poll().expect("timeout never fails") {
                Async::Ready(()) => {
                    // App is probably too slow, but we should process
                    // this situation anyway
                    // Or maybe just convergence_delay is zero
                    if !self.send_current() {
                        return;
                    }
                    self.timer = None;
                }
                Async::NotReady => {
                    self.timer = Some(timeo);
                }
            }
        }
        SubscrFuture::spawn_in(res, self)
    }
    fn poll(&mut self) -> TaskResult {
        let mut updated = false;
        match self.tx.poll_cancel().expect("poll_cancel never fails") {
            Async::Ready(()) => return TaskResult::Stop,
            Async::NotReady => {}
        }
        match self.timer.poll().expect("timeout never fails") {
            Async::Ready(Some(())) => {
                self.timer = None;
                updated = true;
            }
            Async::Ready(None) | Async::NotReady => {}
        }
        match self.input.poll() {
            Err(e) => {
                warn!("Stream of names errored: {}", e);
                return TaskResult::Stop;
            }
            Ok(Async::Ready(None)) => {
                return TaskResult::Stop;
            }
            Ok(Async::NotReady) => {}
            Ok(Async::Ready(Some(x))) => {
                if self.current != x {
                    self.current = x;
                    // restart, so timer is started again
                    return TaskResult::Restart;
                }
            }
        }
        for item in self.items.values_mut() {
            use self::State::*;
            match *item {
                StaticHost(_, _) => {}
                StaticAddr(_) => {}
                Host(ref mut s, ref mut v, _) => {
                    match s.poll() {
                        Err(e) => unreachable(e),
                        Ok(Async::Ready(Some(x))) => {
                            if Some(&x) != v.as_ref() {
                                *v = Some(x);
                                updated = true;
                            }
                        },
                        Ok(Async::Ready(None)) => unreachable!(),
                        Ok(Async::NotReady) => {}
                    }
                }
                Addr(ref mut s, ref mut v) => {
                    match s.poll() {
                        Err(e) => unreachable(e),
                        Ok(Async::Ready(Some(x))) => {
                            if Some(&x) != v.as_ref() {
                                *v = Some(x);
                                updated = true;
                            }
                        },
                        Ok(Async::Ready(None)) => unreachable!(),
                        Ok(Async::NotReady) => {}
                    }
                }
            }
        }
        if updated {
            if self.timer.is_some() {
                if self.items.values().all(|x| x.is_complete()) {
                    self.timer = None;
                }
            }
            if self.timer.is_none() {
                if !self.send_current() {
                    return TaskResult::Stop;
                }
            }
        }
        return TaskResult::Continue;
    }
}
