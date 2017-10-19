use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

use abstract_ns::{Name, Address, IpList};
use abstract_ns::{HostResolve, Resolve, HostSubscribe, Subscribe};
use internal_traits::{Resolver, Wrapper, NullResolver};


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
    pub(crate) suffixes: HashMap<String, Arc<Resolver>>,
    pub(crate) root: Arc<Resolver>,
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
            root: Arc::new(NullResolver),
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

    /// Add a resolver for suffix
    ///
    /// Note: you must supply a full resolver here,
    /// use `null_resolver`/`null_host_resolver` and
    /// [`interval_subscribe`] or `frozen_subscriber`
    /// and other combinators to fullfill needed type.
    ///
    /// [`interval_subscribe`]: trait.SubscribeExt.html#tymethod.interval_subscribe
    pub fn add_suffix<S, R>(&mut self, suffix: S, resolver: R)
        -> &mut Self
        where S: Into<String>,
              R: Resolve + HostResolve + Subscribe + HostSubscribe,
              R: Debug + 'static,
    {
        self.suffixes.insert(suffix.into(),
            Arc::new(Wrapper::new(resolver)));
        self
    }

    /// Removes already configured suffix
    pub fn remove_suffix<S>(&mut self, suffix: &str)
        -> &mut Self
    {
        self.suffixes.remove(suffix);
        self
    }

    /// Adds a host resolver used whenever no suffix matches
    pub fn set_fallthrough<R>(&mut self, resolver: R)
        -> &mut Self
        where R: Resolve + HostResolve + Subscribe + HostSubscribe,
              R: Debug + 'static,
    {
        self.root = Arc::new(Wrapper::new(resolver));
        self
    }

    /// A convenience method that returns Arc'd config
    pub fn done(&self) -> Arc<Config> {
        Arc::new(self.clone())
    }
}
