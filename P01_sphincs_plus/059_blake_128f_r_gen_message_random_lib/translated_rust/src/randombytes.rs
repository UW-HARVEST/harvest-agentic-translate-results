use std::sync::Mutex;
use std::sync::LazyLock;

static FD: LazyLock<Mutex<i32>> = LazyLock::new(|| Mutex::new(-1));

pub fn randombytes(x: &mut [u8], mut xlen: u64) {
    let mut fd = FD.lock().unwrap();
    if *fd == -1 {
        loop {
            let ret = unsafe { libc::open(b"/dev/urandom\0".as_ptr() as *const libc::c_char, libc::O_RDONLY) };
            if ret != -1 {
                *fd = ret;
                break;
            }
            unsafe { libc::sleep(1); }
        }
    }

    let mut offset: usize = 0;
    while xlen > 0 {
        let to_read = if xlen < 1048576 { xlen as usize } else { 1048576 };
        let i = unsafe { libc::read(*fd, x[offset..].as_mut_ptr() as *mut libc::c_void, to_read) };
        if i < 1 {
            unsafe { libc::sleep(1); }
            continue;
        }
        offset += i as usize;
        xlen -= i as u64;
    }
}
