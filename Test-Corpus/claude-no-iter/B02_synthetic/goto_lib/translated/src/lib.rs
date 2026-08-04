use std::ffi::c_char;
use std::ffi::c_int;

use libc::FILE;

fn forward_goto_example(x: c_int) -> c_int {
    unsafe {
        if x < 0 {
            // goto error
            let msg = b"Error: negative input\n\0";
            libc::fprintf(
                libc_stderr(),
                msg.as_ptr() as *const c_char,
            );
            return -1;
        }

        let fmt = b"Processing: %d\n\0";
        libc::printf(fmt.as_ptr() as *const c_char, x);
        x * 2
    }
}

fn open_with_cleanup(filename: *const c_char) -> *mut FILE {
    unsafe {
        let mode = b"r\0";
        let fp: *mut FILE = libc::fopen(filename, mode.as_ptr() as *const c_char);

        if fp.is_null() {
            // goto cleanup
            return cleanup(filename, fp);
        }

        let mut buffer: [c_char; 100] = [0; 100];
        while !libc::fgets(
            buffer.as_mut_ptr(),
            buffer.len() as c_int,
            fp,
        )
        .is_null()
        {
            let fmt = b"%s\0";
            libc::printf(fmt.as_ptr() as *const c_char, buffer.as_ptr());
        }

        if libc::ferror(fp) != 0 {
            // goto cleanup
            return cleanup(filename, fp);
        }

        fp
    }
}

unsafe fn cleanup(filename: *const c_char, fp: *mut FILE) -> *mut FILE {
    let fmt = b"Error: opening or processing file %s\n\0";
    unsafe {
        libc::fprintf(
            libc_stderr(),
            fmt.as_ptr() as *const c_char,
            filename,
        );
        if !fp.is_null() {
            libc::fclose(fp);
        }
    }
    std::ptr::null_mut()
}

#[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd", target_os = "dragonfly")))]
unsafe fn libc_stderr() -> *mut FILE {
    extern "C" {
        static mut stderr: *mut FILE;
    }
    unsafe { stderr }
}

#[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd", target_os = "dragonfly"))]
unsafe fn libc_stderr() -> *mut FILE {
    extern "C" {
        static mut __stderrp: *mut FILE;
    }
    unsafe { __stderrp }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(num: c_int, filename: *const c_char) -> c_int {
    unsafe {
        let res = forward_goto_example(num);
        if res == -1 {
            return -1;
        } else {
            let fmt = b"Goto output: %d\n\0";
            libc::printf(fmt.as_ptr() as *const c_char, res);
        }

        let out = open_with_cleanup(filename);
        if out.is_null() {
            return -2;
        } else {
            libc::fclose(out);
        }

        0
    }
}
