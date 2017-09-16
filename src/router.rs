use std::sync::Arc;

use config::Config;
use futures::Stream;
use futures::stream::iter_ok;
use internal::Table;
use void::Void;


pub struct Router(Arc<Table>);


impl Router {

    /// Create a router for a static config
    pub fn from_config(config: &Arc<Config>) -> Router {
        Router(Arc::new(Table::new(iter_ok(vec![config.clone()]))))
    }

    /// Create a router for a static config
    pub fn from_stream<S>(stream: S) -> Router
        where S: Stream<Item=Arc<Config>, Error=Void>
    {
        Router(Arc::new(Table::new(stream)))
    }
}
