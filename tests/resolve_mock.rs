extern crate abstract_ns;
extern crate futures;
extern crate ns_router;
extern crate tokio_core;

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use futures::{Future, lazy};
use futures::future::{FutureResult, ok};
use abstract_ns::{ResolveHost, Resolve, Name, Address, Error};
use ns_router::{Config, Router};


#[derive(Debug)]
struct Mock;


impl ResolveHost for Mock {
    type FutureHost = FutureResult<Vec<IpAddr>, Error>;
    fn resolve_host(&self, _name: &Name) -> Self::FutureHost {
        ok(vec!["127.0.0.1".parse().unwrap()])
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
        .add_fallthrough_host_resolver(Mock)
        .done();
    let router = Router::from_config(&cfg, &handle);

    // Read first config from a stream
    core.turn(Some(Duration::new(0, 0)));

    let res = core.run(lazy(|| {
        router.resolve_host(&"localhost".parse().unwrap())
    })).unwrap();
    assert_eq!(res, vec!["127.0.0.1".parse::<IpAddr>().unwrap()]);
}

#[test]
fn test_fallback_service() {
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();

    let cfg = Config::new()
        .add_fallthrough_resolver(Mock)
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
