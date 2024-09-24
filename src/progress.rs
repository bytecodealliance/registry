use std::pin::Pin;
use std::task::{ready, Poll};

use indicatif::{ProgressBar, ProgressStyle};
use tokio::io::AsyncRead;

pub(crate) struct ProgressStream<R> {
    bar: ProgressBar,
    inner: R,
}

pub fn make_progress_handler(msg: String) -> impl FnMut(u64, u64) {
    let bar = init_progress_bar();
    bar.set_message(msg);
    move |delta, total| {
        bar.set_length(total);
        bar.inc(delta);
    }
}

fn init_progress_bar() -> ProgressBar {
    let bar = indicatif::ProgressBar::new(0);
    bar.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:30} {binary_bytes:.02}/{binary_total_bytes} {msg}",
        )
        .unwrap()
        .progress_chars("##-"),
    );

    bar
}

impl<R> ProgressStream<R> {
    pub fn new(inner: R, len: u64, msg: String) -> Self {
        let bar = init_progress_bar();
        bar.set_length(len);
        bar.set_message(msg);

        Self { bar, inner }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for ProgressStream<R> {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let initialized_before = buf.initialized().len() as u64;
        match ready!(Pin::new(&mut self.inner).poll_read(cx, buf)) {
            Ok(()) => {
                self.bar
                    .inc(buf.initialized().len() as u64 - initialized_before);
                Poll::Ready(Ok(()))
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}
