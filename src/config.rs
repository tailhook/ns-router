use std::fmt::Debug;
use std::sync::Arc;
use std::net::IpAddr;
use std::collections::HashMap;

use abstract_ns::{Name, Address};
use abstract_ns::{ResolveHost, Resolve, HostSubscribe, Subscribe};
use internal_traits::{Resolver, HostResolver, Subscriber, HostSubscriber};
use internal_traits::{ResolveHostWrapper, ResolveWrapper};
use internal_traits::{HostSubscribeWrapper, SubscribeWrapper};


/// Configuration of the router
///
/// It has a builder interface. You can create a router from `Arc<Config>` or
/// a stream of configs.
#[derive(Clone, Debug)]
pub struct Config {
    pub(crate) hosts: HashMap<Name, Vec<IpAddr>>,
    pub(crate) services: HashMap<Name, Address>,

    pub(crate) suffixes: HashMap<String, Suffix>,

    pub(crate) host_resolver: Option<Arc<HostResolver>>,
    pub(crate) resolver: Option<Arc<Resolver>>,
    pub(crate) host_subscriber: Option<Arc<HostSubscriber>>,
    pub(crate) subscriber: Option<Arc<Subscriber>>,
}

/// Represents configuration of resolvers for a suffix
#[derive(Clone, Debug)]
pub struct Suffix {
    pub(crate) host_resolver: Option<Arc<HostResolver>>,
    pub(crate) resolver: Option<Arc<Resolver>>,
    pub(crate) host_subscriber: Option<Arc<HostSubscriber>>,
    pub(crate) subscriber: Option<Arc<Subscriber>>,
}

impl Config {

    /// Create a new, empty config
    pub fn new() -> Config {
        Config {
            hosts: HashMap::new(),
            services: HashMap::new(),
            suffixes: HashMap::new(),
            host_resolver: None,
            resolver: None,
            host_subscriber: None,
            subscriber: None,
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

    /// Adds or returns configured suffix
    ///
    /// Then you can add suffix-specific resolvers and host-resolvers.
    ///
    /// Note: `add_host` and `add_service` override addresses for specific
    /// hostnames even inside the configured suffix.
    ///
    /// Note: adding a suffix immediately disables fallthrough of the names
    /// matching the suffix to a fallback resolvers. Even if no
    /// resolvers/subscribers are added to the suffix. Use `remove_suffix` or
    /// duplicate fallthrough resolvers here if needed.
    pub fn suffix<S>(&mut self, suffix: S)
        -> &mut Suffix
        where S: Into<String>,
    {
        self.suffixes.entry(suffix.into()).or_insert_with(|| Suffix {
            host_resolver: None,
            resolver: None,
            host_subscriber: None,
            subscriber: None,
        })
    }

    /// Removes already configured suffix
    pub fn remove_suffix<S>(&mut self, suffix: &str)
        -> &mut Self
    {
        self.suffixes.remove(suffix);
        self
    }

    /// Adds a host resolver used whenever no suffix matches
    pub fn set_fallthrough_host_resolver<R>(&mut self, resolver: R)
        -> &mut Self
        where R: ResolveHost + Debug + 'static
    {
        self.host_resolver = Some(Arc::new(
            ResolveHostWrapper::new(resolver)));
        self
    }

    /// Adds a resolver used whenever no suffix matches
    pub fn set_fallthrough_resolver<R>(&mut self, resolver: R)
        -> &mut Self
        where R: Resolve + Debug + 'static
    {
        self.resolver = Some(Arc::new(
            ResolveWrapper::new(resolver)));
        self
    }

    /// A convenience method that returns Arc'd config
    pub fn done(&self) -> Arc<Config> {
        Arc::new(self.clone())
    }
}

impl Suffix {
    /// Sets a host resolver for this suffix
    pub fn set_host_resolver<R>(&mut self, resolver: R)
        -> &mut Self
        where R: ResolveHost + Debug + 'static
    {
        self.host_resolver = Some(Arc::new(
            ResolveHostWrapper::new(resolver)));
        self
    }

    /// Sets a resolver for this suffix
    pub fn set_resolver<R>(&mut self, resolver: R)
        -> &mut Self
        where R: Resolve + Debug + 'static
    {
        self.resolver = Some(Arc::new(
            ResolveWrapper::new(resolver)));
        self
    }

    /// Sets a host subscriber for this suffix
    pub fn set_host_subscriber<S>(&mut self, subscriber: S)
        -> &mut Self
        where S: HostSubscribe + Debug + 'static
    {
        self.host_subscriber = Some(Arc::new(
            HostSubscribeWrapper::new(subscriber)));
        self
    }

    /// Sets a subscriber for this suffix
    pub fn set_subscriber<S>(&mut self, subscriber: S)
        -> &mut Self
        where S: Subscribe + Debug + 'static
    {
        self.subscriber = Some(Arc::new(
            SubscribeWrapper::new(subscriber)));
        self
    }
}
