pub mod s3;
pub mod filesystem;

#[derive(Clone)]
pub enum Kind {
    File,
    Directory,
}