// The following code is adapted from the `mock-tonic` crate:
// https://github.com/tyrchen/tonic-mock
use std::{
    collections::VecDeque,
    pin::Pin,
    task::{Context, Poll},
    vec::Vec,
};

use bytes::{BufMut, Bytes, BytesMut};
use http_body::{Body, Frame};
use prost::Message;
use tonic::Status;

#[derive(Clone)]
pub struct MockBody {
    data: VecDeque<Bytes>,
}

impl MockBody {
    pub fn new(data: Vec<impl Message>) -> Self {
        let mut queue: VecDeque<Bytes> = VecDeque::with_capacity(16);
        for msg in data {
            let buf = Self::encode(&msg);
            queue.push_back(buf);
        }

        MockBody { data: queue }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    // see: https://github.com/hyperium/tonic/blob/1b03ece2a81cb7e8b1922b3c3c1f496bd402d76c/tonic/src/codec/encode.rs#L52
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
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        if self.is_end_stream() {
            Poll::Ready(None)
        } else {
            let msg = self.data.pop_front().unwrap();
            Poll::Ready(Some(Ok(Frame::data(msg))))
        }
    }

    fn is_end_stream(&self) -> bool {
        self.is_empty()
    }

    fn size_hint(&self) -> http_body::SizeHint {
        http_body::SizeHint::with_exact(self.len() as u64)
    }
}
