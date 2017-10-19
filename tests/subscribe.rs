extern crate abstract_ns;
extern crate futures;
extern crate ns_router;
extern crate tokio_core;

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use futures::{lazy};
use futures::future::{Future, Empty, IntoStream, empty};
use futures::future::{FutureResult, ok};
use futures::stream::{once, Stream, Chain, Once};
use abstract_ns::{HostSubscribe, Subscribe, Name, Address, IpList, Error};
use abstract_ns::{Resolve, HostResolve};
use ns_router::{Config, Router};


#[derive(Debug)]
struct Mock;

impl HostResolve for Mock {
    type HostFuture = FutureResult<IpList, Error>;
    fn resolve_host(&self, _name: &Name) -> Self::HostFuture {
        ok(vec!["127.0.0.1".parse().unwrap()].into())
    }
}

impl Resolve for Mock {
    type Future = FutureResult<Address, Error>;
    fn resolve(&self, _name: &Name) -> Self::Future {
        ok(["127.0.0.1:443".parse().unwrap()][..].into())
    }
}

impl HostSubscribe for Mock {
    type HostStream = Chain<Once<IpList, Error>,
                            IntoStream<Empty<IpList, Error>>>;
    type HostError = Error;
    fn subscribe_host(&self, _name: &Name) -> Self::HostStream {
        once(Ok(vec!["127.0.0.1".parse().unwrap()].into()))
            .chain(empty().into_stream())
    }
}

impl Subscribe for Mock {
    type Stream = Chain<Once<Address, Error>,
                            IntoStream<Empty<Address, Error>>>;
    type Error = Error;
    fn subscribe(&self, _name: &Name) -> Self::Stream {
        once(Ok(vec!["127.0.0.1:1234".parse().unwrap()][..].into()))
            .chain(empty().into_stream())
    }
}



#[test]
fn test_overridden_host() {
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();

    let mut cfg = Config::new();
    cfg.add_host(&"localhost".parse().unwrap(),
            vec!["127.0.0.1".parse::<IpAddr>().unwrap()]);
    let (router, up) = Router::updating_config(&cfg.done(), &handle);

    let res = core.run(lazy(|| {
        router.subscribe_host(&"localhost".parse().unwrap()).into_future()
    })).unwrap();
    assert_eq!(res.0,
        Some(vec!["127.0.0.1".parse::<IpAddr>().unwrap()].into()));

    cfg.add_host(&"localhost".parse().unwrap(),
            vec!["127.0.0.2".parse::<IpAddr>().unwrap()]);
    up.update(&cfg.done());

    let res = core.run(res.1.into_future()).unwrap();
    assert_eq!(res.0,
        Some(vec!["127.0.0.2".parse::<IpAddr>().unwrap()].into()));

    let res = core.run(lazy(|| {
        router.subscribe_host(&"localhost".parse().unwrap()).into_future()
    })).unwrap();
    assert_eq!(res.0,
        Some(vec!["127.0.0.2".parse::<IpAddr>().unwrap()].into()));
}

#[test]
fn test_overridden_service() {
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();

    let mut cfg = Config::new();
    cfg.add_service(&"_http._tcp.localhost".parse().unwrap(),
              ["127.0.0.1:80".parse::<SocketAddr>().unwrap()][..].into());
    let (router, up) = Router::updating_config(&cfg.done(), &handle);

    let res = core.run(lazy(|| {
        router.subscribe(
            &"_http._tcp.localhost".parse().unwrap()).into_future()
    })).unwrap();
    assert_eq!(res.0,
        Some(["127.0.0.1:80".parse::<SocketAddr>().unwrap()][..].into()));

    cfg.add_service(&"_http._tcp.localhost".parse().unwrap(),
              ["127.0.0.2:80".parse::<SocketAddr>().unwrap()][..].into());
    up.update(&cfg.done());

    let res = core.run(res.1.into_future()).unwrap();
    assert_eq!(res.0,
        Some(["127.0.0.2:80".parse::<SocketAddr>().unwrap()][..].into()));

    let res = core.run(lazy(|| {
        router.subscribe(
            &"_http._tcp.localhost".parse().unwrap()).into_future()
    })).unwrap();
    assert_eq!(res.0,
        Some(["127.0.0.2:80".parse::<SocketAddr>().unwrap()][..].into()));
}

#[test]
fn test_fallback_host() {
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();

    let cfg = Config::new()
        .set_fallthrough(Mock)
        .done();
    let router = Router::from_config(&cfg, &handle);

    // Read first config from a stream
    core.turn(Some(Duration::new(0, 0)));

    let res = core.run(lazy(|| {
        router.subscribe_host(&"localhost".parse().unwrap()).into_future()
    })).unwrap();
    assert_eq!(res.0,
        Some(vec!["127.0.0.1".parse::<IpAddr>().unwrap()].into()));
}


#[test]
fn test_override_after_fallback_host() {
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();

    let cfg = Config::new()
        .set_fallthrough(Mock)
        .done();
    let (router, up) = Router::updating_config(&cfg.done(), &handle);

    let res = core.run(lazy(|| {
        router.subscribe_host(&"localhost".parse().unwrap()).into_future()
    })).unwrap();
    assert_eq!(res.0,
        Some(vec!["127.0.0.1".parse::<IpAddr>().unwrap()].into()));

    let mut cfg = Config::new();
    cfg.add_host(&"localhost".parse().unwrap(),
            vec!["127.0.0.2".parse::<IpAddr>().unwrap()]);
    up.update(&cfg.done());

    let res = core.run(lazy(|| {
        router.subscribe_host(&"localhost".parse().unwrap()).into_future()
    })).unwrap();
    assert_eq!(res.0,
        Some(vec!["127.0.0.2".parse::<IpAddr>().unwrap()].into()));
}

#[test]
fn test_add_and_override_fallback_service() {
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();

    let mut cfg = Config::new();
    cfg.set_fallthrough(Mock);
    let (router, up) = Router::updating_config(&cfg.done(), &handle);

    let res = core.run(lazy(|| {
        router.subscribe(
            &"_http._tcp.localhost".parse().unwrap()).into_future()
    })).unwrap();
    assert_eq!(res.0,
        Some(["127.0.0.1:1234".parse::<SocketAddr>().unwrap()][..].into()));

    cfg.add_service(&"_http._tcp.localhost".parse().unwrap(),
              ["127.0.0.2:80".parse::<SocketAddr>().unwrap()][..].into());
    up.update(&cfg.done());

    let res = core.run(res.1.into_future()).unwrap();
    assert_eq!(res.0,
        Some(["127.0.0.2:80".parse::<SocketAddr>().unwrap()][..].into()));

    let res = core.run(lazy(|| {
        router.subscribe(
            &"_http._tcp.localhost".parse().unwrap()).into_future()
    })).unwrap();
    assert_eq!(res.0,
        Some(["127.0.0.2:80".parse::<SocketAddr>().unwrap()][..].into()));
}
