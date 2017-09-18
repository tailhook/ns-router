use std::sync::Arc;
use std::net::IpAddr;
use std::collections::HashMap;

use abstract_ns::{Name, Address};


/// Configuration of the router
///
/// It has a builder interface. You can create a router from `Arc<Config>` or
/// a stream of configs.
#[derive(Clone, Debug)]
pub struct Config {
    pub(crate) hosts: HashMap<Name, Vec<IpAddr>>,
    pub(crate) services: HashMap<Name, Address>,
}

impl Config {

    /// Create a new, empty config
    pub fn new() -> Config {
        Config {
            hosts: HashMap::new(),
            services: HashMap::new(),
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

    /// A convenience method that returns Arc'd config
    pub fn done(&self) -> Arc<Config> {
        Arc::new(self.clone())
    }
}

