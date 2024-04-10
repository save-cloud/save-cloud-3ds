use std::ffi::{c_char, c_int, c_longlong, c_void};

pub type HttpProgressCb = extern "C" fn(
    client_ptr: *mut c_void,
    dl_total: c_longlong,
    dl_now: c_longlong,
    ul_total: c_longlong,
    ul_now: c_longlong,
) -> c_int;

extern "C" {
    pub fn http_init();
    pub fn http_exit();
    pub fn http_request(
        method: *const c_char,
        url: *const c_char,
        user_agent: *const c_char,
        body: *const c_char,
        file_to_upload_name: *const c_char,
        file_to_upload_path: *const c_char,
        data_to_upload: *const c_char,
        data_to_upload_len: usize,
        download_file_path: *const c_char,
        ssl_verify: bool,
        progress_cb: Option<HttpProgressCb>,
        client_data_ptr: *mut c_void,
        is_follow: c_int,
    ) -> *mut HttpResponseRaw;
    pub fn http_free_response(response: *mut HttpResponseRaw);
}

pub struct HttpContext;

impl HttpContext {
    pub fn new() -> Self {
        unsafe {
            http_init();
        }
        Self
    }
}

impl Drop for HttpContext {
    fn drop(&mut self) {
        unsafe {
            http_exit();
        }
    }
}

#[repr(C)]
pub struct HttpResponseRaw {
    status: i32,
    message: *const c_char,
    size: usize,
    header_size: usize,
    response: *mut c_char,
    header: *mut c_char,
}
