extern crate tokio_core;
extern crate abstract_ns;
extern crate ns_router;
extern crate ns_std_threaded;

use std::time::Duration;
use std::env;

use abstract_ns::HostResolve;
use ns_router::SubscribeExt;

fn main() {
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let ns = ns_router::Router::from_config(&ns_router::Config::new()
        .set_fallthrough(ns_std_threaded::ThreadedResolver::new()
            .null_service_resolver()
            .interval_subscriber(Duration::new(1, 0), &core.handle()))
        .done(),
        &core.handle());
    for name in env::args().skip(1) {
        let value = core.run(ns.resolve_auto(&name, 80));
        println!("{} resolves to {:?}", name, value);
    }
}
