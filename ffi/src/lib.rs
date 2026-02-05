// Export constants functions
pub use constants::*;

#[unsafe(no_mangle)]
#[allow(nonstandard_style)]
pub extern "C" fn JV_FreeString(ptr: *mut libc::c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(std::ffi::CString::from_raw(ptr));
    }
}
