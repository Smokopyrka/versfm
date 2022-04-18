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
        if split.len() == 1 || split[1] == "" {
            return ("/", split[0]);
        }
        return (split[1], split[0]);
    }

    pub fn append_path_to_dir(dir: &str, path: &str) -> String {
        let mut out = String::new();
        if dir.is_empty() || dir.chars().next().unwrap() != '/' {
            out.push_str("/");
        }
        out.push_str(dir);
        let dir_last_char = dir.chars().last();
        if dir_last_char.is_some() && dir_last_char.unwrap() != '/' {
            out.push_str("/");
        }
        out.push_str(path);
        out
    }
}
