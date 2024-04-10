use std::ffi::{c_char, c_int};

use crate::utils::str_to_c_null_term_bytes;

extern "C" {
    fn loader_launch_file(
        path: *const c_char,
        url: *const c_char,
        return_path: *const c_char,
    ) -> c_int;
}

/// load 3dsx file
///
/// example to load 3dsx file from sdmc and pass args to it:
/// ```
/// loader_file(
///     "sdmc:/3ds/FBI.3dsx",
///     "/save-cloud/cache/url",
///     "sdmc:/3ds/save-cloud.3dsx"
/// );
/// ```
pub fn loader_file(path: &str, path_to_url: &str, return_path: &str) -> bool {
    unsafe {
        let path = str_to_c_null_term_bytes(path);
        let path_to_url = str_to_c_null_term_bytes(path_to_url);
        let return_path = str_to_c_null_term_bytes(return_path);
        loader_launch_file(
            path.as_ptr() as *const c_char,
            path_to_url.as_ptr() as *const c_char,
            return_path.as_ptr() as *const c_char,
        ) >= 0
    }
}
