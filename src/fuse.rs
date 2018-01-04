use futures::{Poll, Async};
use futures::stream::Stream;

/// A stream which "fuse"s a stream once it's terminated.
///
/// Normally streams can behave unpredictably when used after they have already
/// finished, but `Fuse` continues to return `None` from `poll` forever when
/// finished.
///
/// This, is similar to a `futures::stream::Fuse` but also fuses the stream
/// when error occurs.
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct Fuse<S> {
    stream: S,
    done: bool,
}

impl<S> Fuse<S> {
    pub fn new(stream: S) -> Fuse<S> {
        Fuse { stream, done: false }
    }
}

impl<S: Stream> Stream for Fuse<S> {
    type Item = S::Item;
    type Error = S::Error;

    fn poll(&mut self) -> Poll<Option<S::Item>, S::Error> {
        if self.done {
            Ok(Async::Ready(None))
        } else {
            match self.stream.poll() {
                res @ Ok(Async::Ready(None)) | res @ Err(_) => {
                    self.done = true;
                    res
                }
                Ok(Async::Ready(Some(x))) => Ok(Async::Ready(Some(x))),
                Ok(Async::NotReady) => Ok(Async::NotReady),
            }
        }
    }
}

impl<S> Fuse<S> {
    /// Returns whether the underlying stream has finished or not.
    ///
    /// If this method returns `true`, then all future calls to poll are
    /// guaranteed to return `None`. If this returns `false`, then the
    /// underlying stream is still in use.
    pub fn is_done(&self) -> bool {
        self.done
    }
}
