static mut FD: i32 = -1;

extern "C" {
    fn open(path: *const u8, flags: i32) -> i32;
    fn read(fd: i32, buf: *mut u8, count: usize) -> isize;
    fn sleep(seconds: u32) -> u32;
}

const O_RDONLY: i32 = 0;

pub fn randombytes(x: &mut [u8], mut xlen: u64) {
    unsafe {
        if FD == -1 {
            loop {
                FD = open(b"/dev/urandom\0".as_ptr(), O_RDONLY);
                if FD != -1 { break; }
                sleep(1);
            }
        }

        let mut off = 0usize;
        while xlen > 0 {
            let chunk = if xlen < 1048576 { xlen as usize } else { 1048576 };
            let ret = read(FD, x[off..].as_mut_ptr(), chunk);
            if ret < 1 {
                sleep(1);
                continue;
            }
            off += ret as usize;
            xlen -= ret as u64;
        }
    }
}
