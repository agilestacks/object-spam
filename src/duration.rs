use std::time::{Duration, Instant};
use tokio::prelude::*;

pub struct DurationFuture<F> {
    inner: F,
    start: Instant,
}

impl<F> DurationFuture<F>
where
    F: Future,
{
    pub fn new(inner: F) -> Self {
        DurationFuture {
            inner,
            start: Instant::now(),
        }
    }
}

impl<F> Future for DurationFuture<F>
where
    F: Future,
{
    type Item = (F::Item, Duration);
    type Error = F::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.inner.poll().map(|v| match v {
            Async::Ready(v) => Async::Ready((v, self.start.elapsed())),
            Async::NotReady => Async::NotReady,
        })
    }
}

