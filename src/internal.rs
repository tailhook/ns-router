use std::sync::Arc;

use futures::Stream;
use config::Config;
use void::Void;


#[derive(Debug)]
pub struct Table {
}

impl Table {
    pub fn new<S>(stream: S) -> Table
        where S: Stream<Item=Arc<Config>, Error=Void>
    {
        unimplemented!();
    }
}
