use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::c_char;

type ToolBasenameFn = unsafe extern "C" fn(*mut c_char) -> *mut c_char;

const C_LIB_PATH: &str = "c_src/build/libdriver.so";
const RUST_LIB_PATH: &str = "target/debug/libdriver.so";

fn run_case(input: &[u8]) {
    // input is the raw bytes (without trailing NUL); we add NUL
    let cstr_in = CString::new(input).expect("input contains NUL");

    unsafe {
        let c_lib = Library::new(C_LIB_PATH).expect("load C lib");
        let rust_lib = Library::new(RUST_LIB_PATH).expect("load Rust lib");

        let c_fn: Symbol<ToolBasenameFn> =
            c_lib.get(b"tool_basename\0").expect("c sym");
        let rust_fn: Symbol<ToolBasenameFn> =
            rust_lib.get(b"tool_basename\0").expect("rust sym");

        // Each call needs its own buffer; the function returns a pointer
        // into the input buffer, so we compute offsets and compare those.
        let mut buf_c: Vec<u8> = cstr_in.as_bytes_with_nul().to_vec();
        let mut buf_r: Vec<u8> = cstr_in.as_bytes_with_nul().to_vec();

        let base_c = buf_c.as_mut_ptr() as *mut c_char;
        let base_r = buf_r.as_mut_ptr() as *mut c_char;

        let res_c = c_fn(base_c);
        let res_r = rust_fn(base_r);

        let off_c = (res_c as usize).wrapping_sub(base_c as usize);
        let off_r = (res_r as usize).wrapping_sub(base_r as usize);

        assert_eq!(
            off_c, off_r,
            "offset mismatch for input {:?}: C={} Rust={}",
            input, off_c, off_r
        );

        // Also verify the resulting strings are byte-identical.
        let s_c = std::ffi::CStr::from_ptr(res_c).to_bytes().to_vec();
        let s_r = std::ffi::CStr::from_ptr(res_r).to_bytes().to_vec();
        assert_eq!(
            s_c, s_r,
            "result string mismatch for input {:?}",
            input
        );
    }
}

#[test]
fn test_tool_basename_no_separators() {
    run_case(b"filename.txt");
}

#[test]
fn test_tool_basename_empty() {
    run_case(b"");
}

#[test]
fn test_tool_basename_only_slash() {
    run_case(b"/");
}

#[test]
fn test_tool_basename_only_backslash() {
    run_case(b"\\");
}

#[test]
fn test_tool_basename_unix_path() {
    run_case(b"/usr/local/bin/tool");
}

#[test]
fn test_tool_basename_windows_path() {
    run_case(b"C:\\Windows\\System32\\cmd.exe");
}

#[test]
fn test_tool_basename_mixed_slash_then_backslash() {
    run_case(b"/foo/bar\\baz");
}

#[test]
fn test_tool_basename_mixed_backslash_then_slash() {
    run_case(b"C:\\foo\\bar/baz");
}

#[test]
fn test_tool_basename_trailing_slash() {
    run_case(b"/foo/bar/");
}

#[test]
fn test_tool_basename_trailing_backslash() {
    run_case(b"C:\\foo\\bar\\");
}

#[test]
fn test_tool_basename_multiple_slashes() {
    run_case(b"////a");
}

#[test]
fn test_tool_basename_only_file() {
    run_case(b"a");
}

#[test]
fn test_tool_basename_dotfile() {
    run_case(b"/etc/.hidden");
}

#[test]
fn test_tool_basename_relative_unix() {
    run_case(b"./relative/path");
}

#[test]
fn test_tool_basename_relative_windows() {
    run_case(b".\\relative\\path");
}

#[test]
fn test_tool_basename_unc_path() {
    run_case(b"\\\\server\\share\\file");
}

#[test]
fn test_tool_basename_slash_then_backslash_pair() {
    run_case(b"a/b\\");
}

#[test]
fn test_tool_basename_backslash_then_slash_pair() {
    run_case(b"a\\b/");
}

#[test]
fn test_tool_basename_long_path() {
    run_case(b"/a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u/v/w/x/y/z");
}

#[test]
fn test_tool_basename_special_chars() {
    run_case(b"/path with spaces/file (1).tar.gz");
}
