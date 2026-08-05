mod common;
use common::libs;

/// Sanity: both libraries load and report the same version string & numbers.
#[test]
fn version_matches() {
    let l = libs();
    unsafe {
        let (cv, rv) = sympair!(l, b"sodium_version_string", unsafe extern "C" fn() -> *const std::os::raw::c_char);
        let cs = std::ffi::CStr::from_ptr(cv());
        let rs = std::ffi::CStr::from_ptr(rv());
        assert_eq!(cs, rs, "version string");

        let (cmaj, rmaj) = sympair!(l, b"sodium_library_version_major", unsafe extern "C" fn() -> i32);
        assert_eq!(cmaj(), rmaj());
        let (cmin, rmin) = sympair!(l, b"sodium_library_version_minor", unsafe extern "C" fn() -> i32);
        assert_eq!(cmin(), rmin());
    }
}

/// Sanity: a simple one-shot hash matches.
#[test]
fn sha256_matches() {
    let l = libs();
    unsafe {
        let (c, r) = sympair!(
            l,
            b"crypto_hash_sha256",
            unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32
        );
        let input = b"the quick brown fox";
        let mut co = [0u8; 32];
        let mut ro = [0u8; 32];
        let rc = c(co.as_mut_ptr(), input.as_ptr(), input.len() as u64);
        let rr = r(ro.as_mut_ptr(), input.as_ptr(), input.len() as u64);
        assert_eq!(rc, rr);
        assert_eq!(co, ro);
    }
}
