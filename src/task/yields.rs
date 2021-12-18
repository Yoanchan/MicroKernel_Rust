use core::task::{Context, Poll};
use core::{future::Future, pin::Pin};

pub async fn yield_init() {

    Yield(true).await;
}

struct Yield(bool);

impl Future for Yield {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        match self.0 {
            false => Poll::Ready(()),
            true => {
                self.0 = false;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }
}