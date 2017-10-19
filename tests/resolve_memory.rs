extern crate abstract_ns;
extern crate futures;
extern crate ns_router;
extern crate tokio_core;

use std::net::{SocketAddr};
use std::time::Duration;

use abstract_ns::{HostResolve, Resolve, Address, IpList};
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

    let res = core.run(router.resolve_host(&"localhost".parse().unwrap()));
    // Then can query cached hosts immediately
    assert_eq!(res.unwrap(), IpList::parse_list(&["127.0.0.1"]).unwrap());
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
    let res = core.run(
        router.resolve(&"_http._tcp.localhost".parse().unwrap())).unwrap();

    // Then can query cached hosts immediately
    assert_eq!(res, Address::parse_list(&["127.0.0.1:80"]).unwrap());
}

#[test]
fn test_auto() {
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();

    let cfg = Config::new()
        .add_host(&"localhost".parse().unwrap(),
                  vec!["127.0.0.1".parse().unwrap()])
        .add_service(&"_test._tcp.localhost".parse().unwrap(),
                  ["127.0.0.1:8439".parse::<SocketAddr>().unwrap()][..].into())
        .done();
    let router = Router::from_config(&cfg, &handle);

    // Then can query cached hosts immediately
    assert_eq!(
        core.run(router.resolve_auto("localhost:1234", 80)).unwrap(),
        ["127.0.0.1:1234".parse::<SocketAddr>().unwrap()][..].into());
    assert_eq!(
        core.run(router.resolve_auto("localhost", 80)).unwrap(),
        ["127.0.0.1:80".parse::<SocketAddr>().unwrap()][..].into());
    assert_eq!(
        core.run(router.resolve_auto("_test._tcp.localhost", 80)).unwrap(),
        ["127.0.0.1:8439".parse::<SocketAddr>().unwrap()][..].into());
}

#[test]
fn test_straw_addresses() {
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();

    let cfg = Config::new().done();
    let router = Router::from_config(&cfg, &handle);

    // Read first config from a stream
    core.turn(Some(Duration::new(0, 0)));

    // Then can query cached hosts immediately
    assert_eq!(
        core.run(router.resolve_auto("127.0.0.1:1234", 80)).unwrap(),
        ["127.0.0.1:1234".parse::<SocketAddr>().unwrap()][..].into());
    assert_eq!(
        core.run(router.resolve_auto("127.0.0.1", 80)).unwrap(),
        ["127.0.0.1:80".parse::<SocketAddr>().unwrap()][..].into());
    assert_eq!(
        core.run(router.resolve_auto("[2001:db8::2:1]:8123", 80)).unwrap(),
        ["[2001:db8::2:1]:8123".parse::<SocketAddr>().unwrap()][..].into());
    assert_eq!(
        core.run(router.resolve_auto("2001:db8::2:1", 80)).unwrap(),
        ["[2001:db8::2:1]:80".parse::<SocketAddr>().unwrap()][..].into());
}
