use std::fmt;

use abstract_ns::{Name, Error, Address, IpList};
use async_slot as slot;
use futures::sync::oneshot;

use coroutine::{Continuation};


#[derive(Debug)]
pub(crate) enum Request {
    ResolveHost(Name, oneshot::Sender<Result<IpList, Error>>),
    ResolveHostPort(Name, u16, oneshot::Sender<Result<Address, Error>>),
    Resolve(Name, oneshot::Sender<Result<Address, Error>>),
    HostSubscribe(Name, slot::Sender<IpList>),
    Subscribe(Name, slot::Sender<Address>),
    Task(Box<Continuation+Send>),
}

trait AssertTraits: Send {}
impl AssertTraits for Request {}

pub fn reply<X: Send + fmt::Debug + 'static>(name: &Name,
    tx: oneshot::Sender<Result<X, Error>>, value: X)
{
    tx.send(Ok(value))
        .map_err(|value| {
            trace!("{:?} resolved into {:?} but dropped",
                name, value.unwrap());
        })
        .ok();
}

pub fn fail<X: fmt::Debug>(name: &Name,
    tx: oneshot::Sender<Result<X, Error>>, error: Error)
{
    tx.send(Err(error))
        .map_err(|err| {
            trace!("{:?} resolved into, error but dropped: {}",
                name, err.unwrap_err());
        })
        .ok();
}
