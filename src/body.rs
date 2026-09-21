use std::pin::Pin;
use std::task::{Context, Poll, ready};
use std::{fmt, mem};

use bytes::Bytes;
use futures::stream::BoxStream;
use futures::{Stream, StreamExt};
use http_body::{Body as BodyTrait, Frame, SizeHint};

use color_eyre::Report;

pub enum Body {
    Static(Bytes),
    Stream(BoxStream<'static, Bytes>),
}

impl fmt::Debug for Body {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Static(arg0) => f.debug_tuple("Static").field(arg0).finish(),
            Self::Stream(_) => f.debug_tuple("Stream").finish(),
        }
    }
}

impl BodyTrait for Body {
    type Data = Bytes;

    type Error = Report;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.get_mut() {
            Body::Static(bytes) => {
                let b = mem::replace(bytes, Bytes::new());
                Poll::Ready(Some(Ok(Frame::data(b))))
            }
            Body::Stream(stream) => {
                let Some(bytes) = ready!(stream.poll_next_unpin(cx)) else {
                    return Poll::Ready(None);
                };

                Poll::Ready(Some(Ok(Frame::data(bytes))))
            }
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self {
            Body::Static(bytes) => SizeHint::with_exact(bytes.len() as u64),
            Body::Stream(stream) => {
                let (lower, upper) = stream.size_hint();
                let mut size_hint = SizeHint::new();

                size_hint.set_lower(lower as u64);
                if let Some(upper) = upper {
                    size_hint.set_upper(upper as u64);
                }

                size_hint
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        let size_hint = self.size_hint();
        size_hint.upper() == Some(0)
    }
}
