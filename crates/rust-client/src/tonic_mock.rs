// The following code is adapted from the `tonic` crate:
// https://github.com/hyperium/tonic/blob/master/tonic/benches/decode.rs
use std::{
    pin::Pin,
    task::{Context, Poll},
    vec::Vec,
};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use http_body::{Body, Frame, SizeHint};
use prost::Message;
use tonic::Status;

#[derive(Clone)]
pub struct MockBody {
    data: Bytes,
    chunk_size: usize,
}

impl MockBody {
    pub fn new(data: Vec<impl Message>) -> Self {
        let mut queue: Vec<Bytes> = Vec::with_capacity(16);
        for msg in data {
            queue.push(Self::encode(&msg));
        }
        MockBody {
            data: Bytes::from(queue.concat()),
            chunk_size: queue.len(),
        }
    }

    fn encode(msg: &impl Message) -> Bytes {
        let mut buf = BytesMut::with_capacity(256);

        buf.reserve(5);
        unsafe {
            buf.advance_mut(5);
        }
        msg.encode(&mut buf).unwrap();
        {
            let len = buf.len() - 5;
            let mut buf = &mut buf[..5];
            buf.put_u8(0); // byte must be 0, reserve doesn't auto-zero
            buf.put_u32(u32::try_from(len).unwrap());
        }
        buf.freeze()
    }
}

impl Body for MockBody {
    type Data = Bytes;
    type Error = Status;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        if self.data.has_remaining() {
            let split = std::cmp::min(self.chunk_size, self.data.remaining());
            Poll::Ready(Some(Ok(Frame::data(self.data.split_to(split)))))
        } else {
            Poll::Ready(None)
        }
    }

    fn is_end_stream(&self) -> bool {
        !self.data.is_empty()
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::with_exact(self.data.len() as u64)
    }
}
