pub mod providers;
mod view;

pub use view::components;
pub use view::screens;

#[derive(Clone)]
pub enum Kind {
    File,
    Directory,
}

pub mod utils {

    pub fn split_path_into_dir_and_filename(path: &str) -> (&str, &str) {
        let split: Vec<&str> = path.rsplitn(2, "/").collect();
        if split.len() != 2 {
            panic!("Path has no '/' separators in it");
        }
        return (split[1], split[0]);
    }
}
