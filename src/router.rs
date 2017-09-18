use std::sync::Arc;

use futures::{Stream, Future};
use futures::future::{empty};
use futures::stream::{iter_ok, once};
use internal::Table;
use tokio_core::reactor::Handle;
use void::Void;

use config::Config;
use slot;


/// An actual router class
#[derive(Debug, Clone)]
pub struct Router(pub(crate) Arc<Table>);

/// A sink that updates router created using `Router::updating_config`
#[derive(Debug, Clone)]
pub struct UpdateSink(slot::Sender<Arc<Config>>);


impl Router {

    /// Create a router for a static config
    pub fn from_config(config: &Arc<Config>, handle: &Handle) -> Router {
        Router(Table::new(
            once(Ok(config.clone())).chain(empty().into_stream()),
            handle))
    }

    /// Create a router with updating config
    ///
    /// Note: router is defunctional until first config is received in a
    /// stream. By defunctional we mean that every request will wait, until
    /// configured.
    ///
    /// Note 2: when stream is closed router is shut down, so usually the
    /// stream must be infinite.
    pub fn from_stream<S>(stream: S, handle: &Handle) -> Router
        where S: Stream<Item=Arc<Config>, Error=Void> + 'static
    {
        Router(Table::new(stream, handle))
    }

    /// Create a router and update channel
    ///
    /// Note: router is shut down when `UpdateSink` is dropped. So keep
    /// it somewhere so you can update config.
    pub fn updating_config(config: &Arc<Config>, handle: &Handle)
        -> (Router, UpdateSink)
    {
        let (tx, rx) = slot::channel();
        let stream = once(Ok(config.clone())).chain(rx)
            .map_err(|_| unreachable!());
        return (Router(Table::new(stream, handle)), UpdateSink(tx));
    }
}

impl UpdateSink {
    /// Update a config
    ///
    /// Returns `true` if send worked (meaning router is still alive).
    pub fn update(&self, config: &Arc<Config>) -> bool {
        self.0.swap(config.clone()).is_ok()
    }
}
