use std::io;

use bytes::Bytes;
use futures::Stream;

pub mod filesystem;
pub mod s3;

#[derive(Clone)]
pub enum Kind {
    File,
    Directory,
}

pub type BoxedByteStream = Box<dyn Stream<Item = Result<Bytes, io::Error>> + Send + 'static>;
