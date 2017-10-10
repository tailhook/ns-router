extern crate abstract_ns;
extern crate futures;
extern crate ns_router;
extern crate tokio_core;

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use futures::{lazy};
use futures::future::{Future, Empty, FutureResult, IntoStream, ok, empty};
use futures::stream::{once, Stream, Chain, Once};
use abstract_ns::{HostSubscribe, Resolve, Name, Address, Error};
use ns_router::{Config, Router};


#[derive(Debug)]
struct Mock;


impl HostSubscribe for Mock {
    type HostStream = Chain<Once<Vec<IpAddr>, Error>,
                            IntoStream<Empty<Vec<IpAddr>, Error>>>;
    type Error = Error;
    fn subscribe_host(&self, _name: &Name) -> Self::HostStream {
        once(Ok(vec!["127.0.0.1".parse().unwrap()]))
            .chain(empty().into_stream())
    }
}



#[test]
fn test_fallback_host() {
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();

    let cfg = Config::new()
        .set_fallthrough_host_subscriber(Mock)
        .done();
    let router = Router::from_config(&cfg, &handle);

    // Read first config from a stream
    core.turn(Some(Duration::new(0, 0)));

    let res = core.run(lazy(|| {
        router.subscribe_host(&"localhost".parse().unwrap()).into_future()
    })).unwrap();
    assert_eq!(res.0, vec!["127.0.0.1".parse::<IpAddr>().unwrap()]);
}
