use std::fs::File;
use std::io::Read;
use std::os::unix::io::FromRawFd;

static mut FD: i32 = -1;

pub fn randombytes(x: &mut [u8], xlen: u64) {
    let mut remaining = xlen as usize;
    let mut offset = 0usize;

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

        while remaining > 0 {
            let chunk = if remaining < 1048576 { remaining } else { 1048576 };
            let ret = libc::read(FD, x[offset..].as_mut_ptr() as *mut libc::c_void, chunk);
            if ret < 1 {
                libc::sleep(1);
                continue;
            }
            let read_bytes = ret as usize;
            offset += read_bytes;
            remaining -= read_bytes;
        }
    }
}
