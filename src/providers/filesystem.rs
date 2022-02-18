use std::{task::Poll, fs::{File, self}, io::{BufReader, BufRead, self, BufWriter, Write}, self, path::{PathBuf, Path}, borrow::Borrow};

use bytes::Bytes;
use futures::{Stream, StreamExt};

use super::Kind;

#[derive(Clone)]
pub struct FilesystemObject {
    pub name: String,
    pub dir: Option<PathBuf>,
    pub kind: Kind,
}

pub struct FileBytesStream {
    reader: BufReader<File>,
    size: usize, 
}

impl FileBytesStream {
    pub fn new(file: File) -> FileBytesStream {
        let file_len = file.metadata().unwrap().len() as usize;
        FileBytesStream {
            reader: BufReader::new(file),
            size: file_len,
        }
    }
}

impl Stream for FileBytesStream {
    type Item = Result<Bytes, io::Error>;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        match self.reader.fill_buf() {
            Ok(bytes_read) => {
                let consumed = bytes_read.len();
                if consumed > 0 {
                    let bytes_read = Bytes::copy_from_slice(bytes_read);
                    self.reader.consume(consumed);
                    Poll::Ready(Some(Ok(bytes_read)))
                } else {
                    Poll::Ready(None)
                }
            }
            Err(err) => Poll::Ready(Some(Err(err))),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.size, Some(self.size))
    }
}

pub fn get_files_list(path: &Path) -> Vec<FilesystemObject> {
    fs::read_dir(path)
        .unwrap()
        .map(|f| {
            let path = f.unwrap().path();
            let mut file_name = String::from(path.file_name().unwrap().to_str().unwrap());
            let kind: Kind;
            if fs::metadata(&path).unwrap().is_dir() {
                file_name.push_str("/");
                kind = Kind::Directory
            } else {
                kind = Kind::File;
            }
            FilesystemObject {
                name: file_name,
                dir: path.parent().and_then(|p| Some(p.to_path_buf())),
                kind: kind,
            }
        })
        .collect()
}

pub fn get_file_byte_stream(path: &Path) -> FileBytesStream {
    let file = File::open(path).unwrap();
    FileBytesStream::new(file)
}

pub async fn write_file_from_stream<S>(path: &Path, stream: S)
where S: Stream<Item=Result<Bytes, io::Error>> + Send + 'static {
    let mut writer = BufWriter::new(File::create(path).unwrap());
    let mut stream = Box::pin(stream);
    while let Some(chunk) = stream.next().await {
        writer.write(chunk.unwrap().borrow()).unwrap();
    }
}

pub fn remove_file(path: &Path) {
    fs::remove_file(path).unwrap();
}