use crate::bindings::*;
use crate::core::{Status, Pool, NgxStr, OK};
use std::os::raw::{c_void, c_char};
use core::ptr;

#[macro_export]
macro_rules! http_request_handler {
    ( $x: ident, $y: expr ) => {
        #[no_mangle]
        extern "C" fn $x(r: *mut ngx_http_request_t) -> ngx_int_t {
            let status: Status = $y(unsafe { &mut $crate::http::Request::from_ngx_http_request(r) });
            status.0
        }
    };
}

pub struct HTTPStatus(ngx_uint_t);

impl Into<Status> for HTTPStatus {
    fn into(self) -> Status {
        Status(self.0 as ngx_int_t)
    }
}

pub const HTTP_OK: HTTPStatus = HTTPStatus(NGX_HTTP_OK as ngx_uint_t);
pub const HTTP_INTERNAL_SERVER_ERROR: HTTPStatus = HTTPStatus(NGX_HTTP_INTERNAL_SERVER_ERROR as ngx_uint_t);
pub const HTTP_FORBIDDEN: HTTPStatus = HTTPStatus(NGX_HTTP_FORBIDDEN as ngx_uint_t);

pub trait Merge {
    fn merge(&mut self, prev: &Self);
}

impl Merge for () {
    fn merge(&mut self, _prev: &Self) {}
}

pub trait HTTPModule {
    type MainConf: Merge + Default;
    type SrvConf: Merge + Default;
    type LocConf: Merge + Default;

    extern "C" fn preconfiguration(_cf: *mut ngx_conf_t) -> ngx_int_t {
        OK.0
    }

    extern "C" fn postconfiguration(_cf: *mut ngx_conf_t) -> ngx_int_t {
        OK.0
    }

    extern "C" fn create_main_conf(cf: *mut ngx_conf_t) -> *mut c_void {
        let mut pool = unsafe { Pool::from_ngx_pool((*cf).pool) };
        pool.allocate::<Self::MainConf>(Default::default()) as *mut c_void
    }

    extern "C" fn init_main_conf(_cf: *mut ngx_conf_t, _conf: *mut c_void) -> *mut c_char {
        ptr::null_mut()
    }

    extern "C" fn create_srv_conf(cf: *mut ngx_conf_t) -> *mut c_void {
        let mut pool = unsafe { Pool::from_ngx_pool((*cf).pool) };
        pool.allocate::<Self::SrvConf>(Default::default()) as *mut c_void
    }

    extern "C" fn merge_srv_conf(_cf: *mut ngx_conf_t, prev: *mut c_void, conf: *mut c_void) -> *mut c_char {
        let prev = unsafe { &mut *(prev as *mut Self::SrvConf) };
        let conf = unsafe { &mut *(conf as *mut Self::SrvConf) };
        conf.merge(prev);
        ptr::null_mut()
    }

    extern "C" fn create_loc_conf(cf: *mut ngx_conf_t) -> *mut c_void {
        let mut pool = unsafe { Pool::from_ngx_pool((*cf).pool) };
        pool.allocate::<Self::LocConf>(Default::default()) as *mut c_void
    }

    extern "C" fn merge_loc_conf(_cf: *mut ngx_conf_t, prev: *mut c_void, conf: *mut c_void) -> *mut c_char {
        let prev = unsafe { &mut *(prev as *mut Self::LocConf) };
        let conf = unsafe { &mut *(conf as *mut Self::LocConf) };
        conf.merge(prev);
        ptr::null_mut()
    }
}

pub struct Request(*mut ngx_http_request_t);

impl Request {
    pub unsafe fn from_ngx_http_request(r: *mut ngx_http_request_t) -> Request {
        Request(r)
    }

    pub fn is_main(&self) -> bool {
        self.0 == unsafe { (*self.0).main }
    }

    pub fn pool(&self) -> Pool {
        unsafe { Pool::from_ngx_pool((*self.0).pool) }
    }

    pub fn connection(&self) -> *mut ngx_connection_t {
        unsafe { (*self.0).connection }
    }

    pub fn get_module_loc_conf(&self, module: &ngx_module_t) -> *mut c_void {
        unsafe { *(*self.0).loc_conf.offset(module.ctx_index as isize) }
    }

    pub fn get_complex_value(&self, cv: &mut ngx_http_complex_value_t) -> Option<NgxStr> {
        let mut res = ngx_str_t { len: 0, data: ptr::null_mut() };
        unsafe {
            if ngx_http_complex_value(self.0, cv, &mut res) != NGX_OK as ngx_int_t {
                return None;
            }
            Some(NgxStr::from_ngx_str(res))
        }
    }

    pub fn discard_request_body(&mut self) -> Status
    {
        Status(unsafe { ngx_http_discard_request_body(self.0) })
    }

    pub fn user_agent(&self) -> NgxStr {
        unsafe { NgxStr::from_ngx_str((*(*self.0).headers_in.user_agent).value) }
    }

    pub fn set_status(&mut self, status: HTTPStatus) {
        unsafe {
            (*self.0).headers_out.status = status.0;
        }
    }

    pub fn set_content_length_n(&mut self, n: usize) {
        unsafe {
            (*self.0).headers_out.content_length_n = n as off_t;
        }
    }

    pub fn send_header(&self) -> Status {
        Status(unsafe { ngx_http_send_header(self.0) })
    }

    pub fn set_header_only(&self) -> bool {
        unsafe { (*self.0).header_only() != 0 }
    }

    pub fn output_filter(&mut self, body: *mut ngx_chain_t) -> Status {
        Status(unsafe { ngx_http_output_filter(self.0, body) })
    }
}

pub unsafe fn ngx_http_conf_get_module_main_conf(cf: *mut ngx_conf_t, module: &ngx_module_t)  -> *mut c_void {
    let http_conf_ctx = (*cf).ctx as *mut ngx_http_conf_ctx_t;
    *(*http_conf_ctx).main_conf.offset(module.ctx_index as isize)
}

pub unsafe fn ngx_http_conf_get_module_srv_conf(cf: *mut ngx_conf_t, module: &ngx_module_t)  -> *mut c_void {
    let http_conf_ctx = (*cf).ctx as *mut ngx_http_conf_ctx_t;
    *(*http_conf_ctx).srv_conf.offset(module.ctx_index as isize)
}

pub unsafe fn ngx_http_conf_get_module_loc_conf(cf: *mut ngx_conf_t, module: &ngx_module_t)  -> *mut c_void {
    let http_conf_ctx = (*cf).ctx as *mut ngx_http_conf_ctx_t;
    *(*http_conf_ctx).loc_conf.offset(module.ctx_index as isize)
}
