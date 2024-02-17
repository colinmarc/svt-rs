#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[no_mangle]
extern "C" fn __svt_hevc_rust_log_callback(_msg: *const std::ffi::c_char) {
    #[cfg(feature = "log")]
    log::info!("{}", unsafe {
        std::ffi::CStr::from_ptr(_msg)
            .to_str()
            .unwrap()
            .trim_end_matches('\n')
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_handle() {
        simple_logger::init_with_env().ok();

        unsafe {
            let mut foo: *mut EB_COMPONENTTYPE = std::ptr::null_mut();
            let mut cfg = EB_H265_ENC_CONFIGURATION {
                ..std::mem::zeroed()
            };

            let res = EbInitHandle(
                &mut foo as *mut _,
                std::ptr::null_mut(),
                <*mut _>::cast(&mut cfg),
            );

            assert_eq!(0, res);
            assert_eq!(7, cfg.encMode);
        }
    }
}
