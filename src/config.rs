use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

use abstract_ns::{Name, Address, IpList};
use abstract_ns::{ResolveHost, Resolve, HostSubscribe, Subscribe};
use internal_traits::{HostSubscribeWrapper, SubscribeWrapper};
use internal_traits::{ResolveHostWrapper, ResolveWrapper};
use internal_traits::{Resolver, HostResolver, Subscriber, HostSubscriber};


/// Configuration of the router
///
/// It has a builder interface. You can create a router from `Arc<Config>`
/// (made by `Config::done`) or a stream of configs.
#[derive(Clone, Debug)]
pub struct Config {
    pub(crate) restart_delay: Duration,
    pub(crate) convergence_delay: Duration,
    pub(crate) hosts: HashMap<Name, IpList>,
    pub(crate) services: HashMap<Name, Address>,
    pub(crate) suffixes: HashMap<String, Suffix>,
    pub(crate) root: Suffix,
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
            restart_delay: Duration::from_millis(100),
            convergence_delay: Duration::from_millis(100),
            hosts: HashMap::new(),
            services: HashMap::new(),
            suffixes: HashMap::new(),
            root: Suffix {
                host_resolver: None,
                resolver: None,
                host_subscriber: None,
                subscriber: None,
            },
        }
    }

    /// Sets delay after which router will restart any subscription stream
    ///
    /// This works both when stream yields end-of-stream and when stream
    /// has returned error. Default value is 100 milliseconds.
    pub fn restart_delay(&mut self, delay: Duration) -> &mut Self {
        self.restart_delay = delay;
        self
    }

    /// Sets delay used by [`subscribe_many`] family of functions
    ///
    /// The timeout is set when a new set of names arrives via stream or
    /// when configuration is updated. While the timer is active we don't
    /// send name updates to the application unless all (new) names are
    /// resolved.
    ///
    /// Good value is bigger than 90 or 99 percentile of name request
    /// latency, but small enough that delay of this long doesn't introduce
    /// any hiccups on the application.
    ///
    /// For example, if there are names [A, B, C] if C is resolved first
    /// we wait for the 100 millisecond (by default) timer to finish
    /// to let A and B also be resolved. If they aren't within the period
    /// only names in `C` are returned.
    ///
    /// Note if names are reloved later, they are added to the address set
    /// and update is delivered to the client. I.e. it's only important
    /// something depends on the first address value.
    ///
    /// The case where it's important is following: Client establishes
    /// few persistent connections by picking random IP Addresses from the
    /// set. Once new addresses arrive connections are still hold until
    /// one of them is broken. In this case, the first address (or the
    /// one that can be resolved faster for any reason) would have more
    /// connections received in most cases which might be a problem.
    ///
    /// [`subscribe_many`]: struct.Router.html#tymethod.subscribe_many
    pub fn convergence_delay(&mut self, delay: Duration) -> &mut Self {
        self.convergence_delay = delay;
        self
    }

    /// Add a host that will be resolved to list of addreses
    ///
    /// Hosts added by this host method overrides any other resolvers.
    pub fn add_host<A>(&mut self, name: &Name, addr: A)
        -> &mut Self
        where A: Into<IpList>
    {
        self.hosts.insert(name.clone(), addr.into());
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
        self.root.host_resolver = Some(Arc::new(
            ResolveHostWrapper::new(resolver)));
        self
    }

    /// Adds a resolver used whenever no suffix matches
    pub fn set_fallthrough_resolver<R>(&mut self, resolver: R)
        -> &mut Self
        where R: Resolve + Debug + 'static
    {
        self.root.resolver = Some(Arc::new(
            ResolveWrapper::new(resolver)));
        self
    }

    /// Sets a host subscriber used whenever no suffix matches
    pub fn set_fallthrough_host_subscriber<S>(&mut self, subscriber: S)
        -> &mut Self
        where S: HostSubscribe + Debug + 'static
    {
        self.root.host_subscriber = Some(Arc::new(
            HostSubscribeWrapper::new(subscriber)));
        self
    }

    /// Sets a subscriber used whenever no suffix matches
    pub fn set_fallthrough_subscriber<S>(&mut self, subscriber: S)
        -> &mut Self
        where S: Subscribe + Debug + 'static
    {
        self.root.subscriber = Some(Arc::new(
            SubscribeWrapper::new(subscriber)));
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
