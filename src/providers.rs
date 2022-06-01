//! Module defining providers used for integrating with various
//! filesystems, object stores, etc.
use std::io;

use bytes::Bytes;
use futures::Stream;

pub mod filesystem;
pub mod s3;

/// Enum representing the possible kinds of files
///
/// * `File` - Regular File
/// * `Directory` - Directory File
/// * `Unknown` - File of unknown type (possibly a result
/// of the program not being able to read its metadata)
#[derive(Clone)]
pub enum Kind {
    File,
    Directory,
    Unknown,
}

pub type BoxedByteStream = Box<dyn Stream<Item = Result<Bytes, io::Error>> + Send + 'static>;
