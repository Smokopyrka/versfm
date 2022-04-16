pub mod providers;
mod view;

pub use view::components;
pub use view::screens;

#[derive(Clone)]
pub enum Kind {
    File,
    Directory,
}
