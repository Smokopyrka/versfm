pub mod filesystem;
pub mod s3;

#[derive(Clone)]
pub enum Kind {
    File,
    Directory,
}
