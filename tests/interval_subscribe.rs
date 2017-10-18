extern crate abstract_ns;
extern crate futures;
extern crate ns_router;
extern crate tokio_core;

use std::net::{IpAddr};
use std::time::Duration;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures::{Stream, lazy};
use futures::future::{FutureResult, ok};
use abstract_ns::{HostSubscribe, Subscribe, Name, Address, IpList, Error};
use abstract_ns::{Resolve, ResolveHost};
use ns_router::{Config, Router, SubscribeExt};

#[derive(Debug)]
struct IncrMock(AtomicUsize);

impl ResolveHost for IncrMock {
    type FutureHost = FutureResult<IpList, Error>;
    fn resolve_host(&self, _name: &Name) -> Self::FutureHost {
        let n = self.0.fetch_add(1, Ordering::SeqCst);
        ok(vec![format!("127.0.0.{}", n).parse().unwrap()].into())
    }
}

impl Resolve for IncrMock {
    type Future = FutureResult<Address, Error>;
    fn resolve(&self, _name: &Name) -> Self::Future {
        let n = self.0.fetch_add(1, Ordering::SeqCst);
        ok([format!("127.0.0.{}:443", n).parse().unwrap()][..].into())
    }
}

#[test]
fn test_host() {
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();

    let cfg = Config::new()
        .set_fallthrough(IncrMock(AtomicUsize::new(1))
            .interval_subscriber(Duration::from_millis(100), &handle))
        .done();
    let router = Router::from_config(&cfg, &handle);

    let res = core.run(lazy(|| {
        router.subscribe_host(&"localhost".parse().unwrap()).into_future()
    })).unwrap();
    assert_eq!(res.0,
        Some(vec!["127.0.0.1".parse::<IpAddr>().unwrap()].into()));

    let res = core.run(lazy(move || res.1.into_future())).unwrap();
    assert_eq!(res.0,
        Some(vec!["127.0.0.2".parse::<IpAddr>().unwrap()].into()));
}

#[test]
fn test_service() {
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();

    let cfg = Config::new()
        .set_fallthrough(IncrMock(AtomicUsize::new(1))
            .interval_subscriber(Duration::from_millis(100), &handle))
        .done();
    let router = Router::from_config(&cfg, &handle);

    let res = core.run(lazy(|| {
        router.subscribe(&"localhost".parse().unwrap()).into_future()
    })).unwrap();
    assert_eq!(res.0,
        Some(Address::parse_list(&["127.0.0.1:443"]).unwrap()));

    println!("First time");

    let res = core.run(lazy(move || res.1.into_future())).unwrap();
    assert_eq!(res.0,
        Some(Address::parse_list(&["127.0.0.2:443"]).unwrap()));
}
