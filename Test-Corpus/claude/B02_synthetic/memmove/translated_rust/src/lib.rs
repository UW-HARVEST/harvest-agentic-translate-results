// Library entry: re-exports the buffer processing logic and provides FFI
// shims that match the C ABI exactly so external callers can link against
// this Rust .so as a drop-in replacement for the C .so.

pub mod lib_buffer;

use core::ffi::c_int;

/// FFI wrapper exposing `process_buffer` with the exact C signature.
///
/// C: `size_t process_buffer(uint8_t *buffer, size_t length, uint32_t flags,
///                           int param1, int param2);`
///
/// # Safety
/// Caller must ensure `buffer` points to at least `length * 2 + 64` writable
/// bytes when `length > 0`. The C implementation does internal `memmove`
/// operations that may transiently access indices well beyond `length` (notably
/// in `compact_runs` when the threshold is small). Rust's bounds-checked slice
/// model requires the backing slice to be large enough to cover those accesses,
/// so the FFI wrapper constructs a slice that matches the implicit C contract.
#[no_mangle]
pub unsafe extern "C" fn process_buffer(
    buffer: *mut u8,
    length: usize,
    flags: u32,
    param1: c_int,
    param2: c_int,
) -> usize {
    if length == 0 {
        return 0;
    }
    if buffer.is_null() {
        return 0;
    }
    // The C implementation can transiently grow `len` up to 2 * length during
    // `compact_runs` (each input byte may expand to two output bytes when
    // threshold is small). Add a small safety pad so that `copy_within`
    // destinations stay in-bounds for the worst case.
    let slice_len = length.saturating_mul(2).saturating_add(64);
    let slice = core::slice::from_raw_parts_mut(buffer, slice_len);
    lib_buffer::process_buffer(slice, length, flags, param1 as i32, param2 as i32)
}
