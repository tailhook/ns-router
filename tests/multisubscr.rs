extern crate abstract_ns;
extern crate futures;
extern crate ns_router;
extern crate tokio_core;

use std::net::{IpAddr, SocketAddr};

use futures::{lazy};
use futures::future::{Future, Empty, IntoStream, empty};
use futures::stream::{once, Stream, Chain, Once};
use abstract_ns::{HostSubscribe, Subscribe, Name, Address, IpList, Error};
use ns_router::{Config, Router};


#[derive(Debug)]
struct Mock;


impl HostSubscribe for Mock {
    type HostStream = Chain<Once<IpList, Error>,
                            IntoStream<Empty<IpList, Error>>>;
    type Error = Error;
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
fn host_and_service() {
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();

    let mut cfg = Config::new();
    cfg.add_host(&"example.org".parse().unwrap(),
                 vec!["127.0.0.2".parse::<IpAddr>().unwrap()]);
    cfg.set_fallthrough_subscriber(Mock);
    let (router, up) = Router::updating_config(&cfg.done(), &handle);

    let res = core.run(lazy(|| {
        router.subscribe_many(&[
            "example.org",
            "_http._tcp.localhost",
        ], 8080).into_future()
    })).unwrap();
    assert_eq!(res.0,
        Some([
            "127.0.0.1:1234".parse::<SocketAddr>().unwrap(),
            "127.0.0.2:8080".parse::<SocketAddr>().unwrap(),
        ][..].into()));

    cfg.add_service(&"_http._tcp.localhost".parse().unwrap(),
              ["127.0.0.3:80".parse::<SocketAddr>().unwrap()][..].into());
    up.update(&cfg.done());

    let res = core.run(res.1.into_future()).unwrap();
    assert_eq!(res.0,
        Some([
            "127.0.0.3:80".parse::<SocketAddr>().unwrap(),
            "127.0.0.2:8080".parse::<SocketAddr>().unwrap(),
        ][..].into()));
}
