//! A versatile file manager that integrates with various data stores
//! such as you local file system or an AWS S3 Bucket
pub mod providers;
mod view;

pub use view::components;
pub use view::screens;

/// Module defining utility functions for operating on paths represented
/// as strings
pub mod utils {

    /// Splits given path into its directory and filename component
    ///
    /// # Arguments
    ///
    /// * `path` - A string representing the full file path
    ///
    /// # Examples
    ///
    /// ```
    /// use versfm::utils::split_path_into_dir_and_filename;
    /// let (dir, file_name) = split_path_into_dir_and_filename("/home/user/some_file.txt");
    /// assert_eq!(dir, "/home/user");
    /// assert_eq!(file_name, "some_file.txt");
    /// ```
    pub fn split_path_into_dir_and_filename(path: &str) -> (&str, &str) {
        let split: Vec<&str> = path.rsplitn(2, "/").collect();
        if split.len() == 1 || split[1] == "" {
            return ("/", split[0]);
        }
        return (split[1], split[0]);
    }

    /// Appends given path to the end of the base path
    ///
    /// # Arguments
    ///
    /// * `base` - The base path to which the `path` should be
    /// appended
    /// * `path` - The path that should be appended
    ///
    /// # Examples
    ///
    /// ```
    /// use versfm::utils::append_path_to_dir;
    /// assert_eq!(
    ///     append_path_to_dir("/home/user/", "some_path"),
    ///     "/home/user/some_path"
    /// );
    ///
    /// ```
    /// **Ommiting the first or last '/' character in a base path
    /// is valid. The function will produce the same result regardless**
    ///
    /// * Ommiting the last '/' character
    /// ```
    /// use versfm::utils::append_path_to_dir;
    /// assert_eq!(
    ///     append_path_to_dir("/home/user", "some_path"),
    ///     "/home/user/some_path"
    /// );
    /// ```
    /// * Ommiting the first '/' character
    /// ```
    /// use versfm::utils::append_path_to_dir;
    /// assert_eq!(
    ///     append_path_to_dir("home/user/", "some_path"),
    ///     "/home/user/some_path"
    /// );
    /// ```
    /// * Ommiting both the first and the last '/' characters
    /// ```
    /// use versfm::utils::append_path_to_dir;
    /// assert_eq!(
    ///     append_path_to_dir("home/user", "some_path"),
    ///     "/home/user/some_path"
    /// );
    /// ```
    pub fn append_path_to_dir(base: &str, path: &str) -> String {
        let mut out = String::new();
        if base.is_empty() || base.chars().next().unwrap() != '/' {
            out.push_str("/");
        }
        out.push_str(base);
        let dir_last_char = base.chars().last();
        if dir_last_char.is_some() && dir_last_char.unwrap() != '/' {
            out.push_str("/");
        }
        out.push_str(path);
        out
    }
}
