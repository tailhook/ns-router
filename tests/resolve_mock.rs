extern crate abstract_ns;
extern crate futures;
extern crate ns_router;
extern crate tokio_core;

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use futures::{lazy};
use futures::future::{FutureResult, ok};
use abstract_ns::{HostResolve, Resolve, Name, Address, IpList, Error};
use ns_router::{Config, Router};


#[derive(Debug)]
struct Mock;

#[derive(Debug)]
struct Mock2;


impl HostResolve for Mock {
    type HostFuture = FutureResult<IpList, Error>;
    fn resolve_host(&self, _name: &Name) -> Self::HostFuture {
        ok(vec!["127.0.0.1".parse().unwrap()].into())
    }
}

impl HostResolve for Mock2 {
    type HostFuture = FutureResult<IpList, Error>;
    fn resolve_host(&self, _name: &Name) -> Self::HostFuture {
        ok(vec!["127.0.0.2".parse().unwrap()].into())
    }
}

impl Resolve for Mock2 {
    type Future = FutureResult<Address, Error>;
    fn resolve(&self, _name: &Name) -> Self::Future {
        ok(["127.0.0.2:443".parse().unwrap()][..].into())
    }
}

impl Resolve for Mock {
    type Future = FutureResult<Address, Error>;
    fn resolve(&self, _name: &Name) -> Self::Future {
        ok(["127.0.0.1:443".parse().unwrap()][..].into())
    }
}


#[test]
fn test_fallback_host() {
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();

    let cfg = Config::new()
        .set_fallthrough(Mock.frozen_subscriber())
        .done();
    let router = Router::from_config(&cfg, &handle);

    // Read first config from a stream
    core.turn(Some(Duration::new(0, 0)));

    let res = core.run(lazy(|| {
        router.resolve_host(&"localhost".parse().unwrap())
    })).unwrap();
    assert_eq!(res, vec!["127.0.0.1".parse::<IpAddr>().unwrap()].into());
}

#[test]
fn test_fallback_service() {
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();

    let cfg = Config::new()
        .set_fallthrough(Mock.frozen_subscriber())
        .done();
    let router = Router::from_config(&cfg, &handle);

    // Read first config from a stream
    core.turn(Some(Duration::new(0, 0)));

    let res = core.run(lazy(|| {
        router.resolve(&"_tcp._xmpp-server.localhost".parse().unwrap())
    })).unwrap();
    assert_eq!(res,
        ["127.0.0.1:443".parse::<SocketAddr>().unwrap()][..].into());
}

#[test]
fn test_host_suffix() {
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();

    let cfg = Config::new()
        .add_suffix("consul", Mock2.frozen_subscriber())
        .set_fallthrough(Mock.frozen_subscriber())
        .done();
    let router = Router::from_config(&cfg, &handle);

    // Read first config from a stream
    core.turn(Some(Duration::new(0, 0)));

    let res = core.run(lazy(|| {
        router.resolve_host(&"x.consul".parse().unwrap())
    })).unwrap();
    assert_eq!(res, vec!["127.0.0.2".parse::<IpAddr>().unwrap()].into());

    let res = core.run(lazy(|| {
        router.resolve_host(&"localhost".parse().unwrap())
    })).unwrap();
    assert_eq!(res, vec!["127.0.0.1".parse::<IpAddr>().unwrap()].into());
}

#[test]
fn test_suffix() {
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();

    let cfg = Config::new()
        .add_suffix("consul", Mock2.frozen_subscriber())
        .set_fallthrough(Mock.frozen_subscriber())
        .done();
    let router = Router::from_config(&cfg, &handle);

    // Read first config from a stream
    core.turn(Some(Duration::new(0, 0)));

    let res = core.run(lazy(|| {
        router.resolve(&"x.consul".parse().unwrap())
    })).unwrap();
    assert_eq!(res,
        ["127.0.0.2:443".parse::<SocketAddr>().unwrap()][..].into());

    let res = core.run(lazy(|| {
        router.resolve(&"localhost".parse().unwrap())
    })).unwrap();
    assert_eq!(res,
        ["127.0.0.1:443".parse::<SocketAddr>().unwrap()][..].into());
}
