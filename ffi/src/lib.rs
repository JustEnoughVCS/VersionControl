// Export constants functions
pub use constants::*;

#[unsafe(no_mangle)]
#[allow(nonstandard_style)]
/// # Safety
///
/// This function must be called with a pointer that was obtained from
/// `JV_Const_*` functions or `std::ffi::CString::into_raw()`. The pointer
/// must not be null unless it was explicitly returned as null from the
/// allocating function. After calling this function, the pointer becomes
/// invalid and must not be used again.
pub unsafe extern "C" fn JV_FreeString(ptr: *mut libc::c_char) {
    unsafe {
        if ptr.is_null() {
            return;
        }
        drop(std::ffi::CString::from_raw(ptr));
    }
}
