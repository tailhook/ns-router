use std::sync::Arc;

use futures::Stream;
use futures::stream::iter_ok;
use internal::Table;
use void::Void;

use config::Config;
use slot;


/// An actual router class
#[derive(Debug, Clone)]
pub struct Router(Arc<Table>);

/// A sink that updates router created using `Router::updating_config`
#[derive(Debug, Clone)]
pub struct UpdateSink(slot::Sender<Arc<Config>>);


impl Router {

    /// Create a router for a static config
    pub fn from_config(config: &Arc<Config>) -> Router {
        Router(Arc::new(Table::new(iter_ok(vec![config.clone()]))))
    }

    /// Create a router with updating config
    ///
    /// Note: router is defunctional until first config is received in a
    /// stream. By defunctional we mean that every request will wait, until
    /// configured.
    pub fn from_stream<S>(stream: S) -> Router
        where S: Stream<Item=Arc<Config>, Error=Void>
    {
        Router(Arc::new(Table::new(stream)))
    }

    /// Create a router and update channel
    pub fn updating_config(config: &Arc<Config>) -> (Router, UpdateSink) {
        let (tx, rx) = slot::channel();
        let stream = iter_ok(vec![config.clone()]).chain(rx)
            .map_err(|_| unreachable!());
        return (Router(Arc::new(Table::new(stream))), UpdateSink(tx));
    }
}

impl UpdateSink {
    /// Update a config
    ///
    /// Returns `true` if send worked (meaning router is still alive).
    fn update(&self, config: &Arc<Config>) -> bool {
        self.0.swap(config.clone()).is_ok()
    }
}
