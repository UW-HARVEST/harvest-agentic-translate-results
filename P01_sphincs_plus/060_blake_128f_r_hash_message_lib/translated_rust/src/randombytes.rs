use std::fs::File;
use std::io::Read;
use std::os::unix::io::FromRawFd;

static mut FD: i32 = -1;

pub fn randombytes(x: &mut [u8], mut xlen: u64) {
    unsafe {
        if FD == -1 {
            loop {
                FD = libc::open(b"/dev/urandom\0".as_ptr() as *const libc::c_char, libc::O_RDONLY);
                if FD != -1 {
                    break;
                }
                libc::sleep(1);
            }
        }

        let mut off: usize = 0;
        while xlen > 0 {
            let chunk = if xlen < 1048576 { xlen } else { 1048576 };
            let i = libc::read(FD, x[off..].as_mut_ptr() as *mut libc::c_void, chunk as usize);
            if i < 1 {
                libc::sleep(1);
                continue;
            }
            off += i as usize;
            xlen -= i as u64;
        }
    }
}
