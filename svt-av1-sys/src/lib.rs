#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[no_mangle]
#[cfg(feature = "log")]
extern "C" fn __svt_av1_rust_log_callback(
    level: std::ffi::c_int,
    tag: *const std::ffi::c_char,
    msg: *const std::ffi::c_char,
) {
    let level = match level {
        0 | 1 => log::Level::Error,
        2 => log::Level::Warn,
        3 | -1 => log::Level::Info,
        4 => log::Level::Debug,
        _ => return,
    };

    let tag = unsafe { std::ffi::CStr::from_ptr(tag).to_str().unwrap() };
    let msg = unsafe {
        std::ffi::CStr::from_ptr(msg)
            .to_str()
            .unwrap()
            .trim_end_matches('\n')
    };

    log::log!(level, "{}: {}", tag, msg);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_handle() {
        simple_logger::init_with_env().ok();

        unsafe {
            let mut foo: *mut EbComponentType = std::ptr::null_mut();
            let mut cfg = EbSvtAv1EncConfiguration {
                ..std::mem::zeroed()
            };

            let res = svt_av1_enc_init_handle(
                &mut foo as *mut _,
                std::ptr::null_mut(),
                <*mut _>::cast(&mut cfg),
            );

            assert_eq!(0, res);
            assert_eq!(10, cfg.enc_mode);
        }
    }
}
