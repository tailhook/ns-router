extern crate abstract_ns;
extern crate futures;
extern crate ns_router;
extern crate tokio_core;

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use futures::Future;
use abstract_ns::{ResolveHost, Resolve};
use ns_router::{Config, Router};



#[test]
fn test_host() {
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();

    let cfg = Config::new()
        .add_host(&"localhost".parse().unwrap(),
                  vec!["127.0.0.1".parse().unwrap()])
        .done();
    let router = Router::from_config(&cfg, &handle);

    // Read first config from a stream
    core.turn(Some(Duration::new(0, 0)));

    // Then can query cached hosts immediately
    assert_eq!(
        router.resolve_host(&"localhost".parse().unwrap()).wait().unwrap(),
        vec!["127.0.0.1".parse::<IpAddr>().unwrap()].into());
}

#[test]
fn test_addr() {
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();

    let cfg = Config::new()
        .add_service(&"_http._tcp.localhost".parse().unwrap(),
                  ["127.0.0.1:80".parse::<SocketAddr>().unwrap()][..].into())
        .done();
    let router = Router::from_config(&cfg, &handle);

    // Read first config from a stream
    core.turn(Some(Duration::new(0, 0)));

    // Then can query cached hosts immediately
    assert_eq!(
        router.resolve(&"_http._tcp.localhost".parse().unwrap())
            .wait().unwrap(),
        ["127.0.0.1:80".parse::<SocketAddr>().unwrap()][..].into());
}
