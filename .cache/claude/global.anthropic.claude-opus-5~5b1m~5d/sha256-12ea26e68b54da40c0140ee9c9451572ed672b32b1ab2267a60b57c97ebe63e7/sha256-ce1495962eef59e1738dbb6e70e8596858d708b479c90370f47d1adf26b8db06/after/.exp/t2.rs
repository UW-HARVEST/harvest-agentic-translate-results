use std::ffi::{c_char, c_void, c_int};
extern "C" {
    static mut stdout: *mut c_void;
    fn fopen(p: *const c_char, m: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
    fn dlopen(f: *const c_char, flag: c_int) -> *mut c_void;
    fn dlsym(h: *mut c_void, s: *const c_char) -> *mut c_void;
}
fn main() {
    let so = std::env::args().nth(1).unwrap();
    let path = std::env::args().nth(2).unwrap();
    unsafe {
        let h = dlopen(format!("{so}\0").as_ptr() as *const c_char, 2);
        assert!(!h.is_null(), "dlopen failed");
        let pl: unsafe extern "C" fn(*const c_char) = std::mem::transmute(dlsym(h, b"printLine\0".as_ptr() as *const c_char));
        let drv: unsafe extern "C" fn() = std::mem::transmute(dlsym(h, b"driver\0".as_ptr() as *const c_char));
        let cpath = format!("{}\0", path);
        let f = fopen(cpath.as_ptr() as *const c_char, b"w\0".as_ptr() as *const c_char);
        assert!(!f.is_null());
        let saved = std::ptr::read(std::ptr::addr_of!(stdout));
        std::ptr::write(std::ptr::addr_of_mut!(stdout), f);
        pl(b"hello from swapped stdout\0".as_ptr() as *const c_char);
        drv();
        fflush(f);
        std::ptr::write(std::ptr::addr_of_mut!(stdout), saved);
        fclose(f);
    }
    println!("--- terminal output OK ---");
    println!("captured:\n{}", std::fs::read_to_string(&path).unwrap());
}
