mod common;
use common::*;

#[test]
fn smoke_load_both() {
    let l = libs();
    unsafe {
        (l.c.rand_seed)(0x31415926);
        (l.r.rand_seed)(0x31415926);
        let data = b"hello world";
        let ch = (l.c.hash_bytes)(data.as_ptr() as *mut _, data.len(), 12345);
        let rh = (l.r.hash_bytes)(data.as_ptr() as *mut _, data.len(), 12345);
        assert_eq!(ch, rh, "hash_bytes mismatch");
        let s = cs("hello");
        let ch = (l.c.hash_string)(s.as_ptr() as *mut _, 7);
        let rh = (l.r.hash_string)(s.as_ptr() as *mut _, 7);
        assert_eq!(ch, rh, "hash_string mismatch");
    }
}

#[test]
fn smoke_str_put() {
    let l = libs();
    for num in [0i32, 1, 2, 5, 100] {
        let cout = capture_stdout(|| unsafe { (l.c.rand_seed)(0x31415926); (l.c.str_put)(num) });
        let rout = capture_stdout(|| unsafe { (l.r.rand_seed)(0x31415926); (l.r.str_put)(num) });
        assert_eq!(
            String::from_utf8_lossy(&cout),
            String::from_utf8_lossy(&rout),
            "str_put({num}) stdout mismatch"
        );
        eprintln!("str_put({num}) => {:?}", String::from_utf8_lossy(&cout));
    }
}
