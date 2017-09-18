use std::fmt::Debug;
use std::sync::Arc;
use std::net::IpAddr;
use std::collections::HashMap;

use abstract_ns::{Name, Address};
use abstract_ns::{ResolveHost};
use internal_traits::{Resolver, HostResolver, Subscriber, HostSubscriber};
use internal_traits::{ResolveHostWrapper};


/// Configuration of the router
///
/// It has a builder interface. You can create a router from `Arc<Config>` or
/// a stream of configs.
#[derive(Clone, Debug)]
pub struct Config {
    pub(crate) hosts: HashMap<Name, Vec<IpAddr>>,
    pub(crate) services: HashMap<Name, Address>,
    pub(crate) host_resolver: Option<Arc<Resolver>>,
}

impl Config {

    /// Create a new, empty config
    pub fn new() -> Config {
        Config {
            hosts: HashMap::new(),
            services: HashMap::new(),
            host_resolver: None,
        }
    }

    /// Add a host that will be resolved to list of addreses
    ///
    /// Hosts added by this host method overrides any other resolvers.
    pub fn add_host(&mut self, name: &Name, addr: Vec<IpAddr>) -> &mut Self {
        self.hosts.insert(name.clone(), addr);
        self
    }

    /// Add a service that will be resolved to an Address object
    ///
    /// Service names added by this host method overrides any other resolvers.
    pub fn add_service(&mut self, name: &Name, addr: Address) -> &mut Self {
        self.services.insert(name.clone(), addr);
        self
    }

    /// Adds a host resolver used whenever no suffix matches
    pub fn add_fallthrough_host_resolver<R>(&mut self, resolver: R)
        where R: ResolveHost + Debug + 'static
    {
        self.host_resolver = Some(Arc::new(
            ResolveHostWrapper::new(resolver)));
    }

    /// A convenience method that returns Arc'd config
    pub fn done(&self) -> Arc<Config> {
        Arc::new(self.clone())
    }
}

