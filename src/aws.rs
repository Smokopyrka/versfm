use crate::view::components::{FilesystemList, FilesystemObject};

use self::s3::S3Object;

pub mod s3;

#[derive(Clone)]
pub enum Kind {
    File,
    Directory,
}

// impl From<&S3Object> for FileEntry {
//     fn from(entry: &S3Object) -> Self {
//         FileEntry {
//             name: entry.name,
//             kind: entry.kind,
//         }
//     }
// }

// impl From<&FilesystemObject> for FileEntry {
//     fn from(entry: &FilesystemObject) -> Self {
//         FileEntry {
//             name: entry.name,
//             kind: entry.kind,
//         }
//     }
// }
