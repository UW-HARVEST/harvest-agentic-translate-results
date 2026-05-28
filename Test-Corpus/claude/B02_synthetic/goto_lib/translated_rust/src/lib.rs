use std::ffi::c_char;
use std::ffi::c_int;
use std::ptr;

extern "C" {
    static stderr: *mut libc::FILE;
}

#[unsafe(no_mangle)]
pub extern "C" fn forward_goto_example(x: c_int) -> c_int {
    unsafe {
        if x < 0 {
            // goto error;
            let msg = b"Error: negative input\n\0";
            libc::fprintf(
                stderr,
                msg.as_ptr() as *const c_char,
            );
            return -1;
        }

        let fmt = b"Processing: %d\n\0";
        libc::printf(fmt.as_ptr() as *const c_char, x);
        x * 2
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn open_with_cleanup(filename: *const c_char) -> *mut libc::FILE {
    unsafe {
        let mode = b"r\0";
        let fp = libc::fopen(filename, mode.as_ptr() as *const c_char);
        if fp.is_null() {
            // goto cleanup
            let err_fmt = b"Error: opening or processing file %s\n\0";
            libc::fprintf(stderr, err_fmt.as_ptr() as *const c_char, filename);
            // if(fp) fclose(fp); -- fp is null so don't close
            return ptr::null_mut();
        }

        let mut buffer: [c_char; 100] = [0; 100];
        while !libc::fgets(
            buffer.as_mut_ptr(),
            buffer.len() as c_int,
            fp,
        )
        .is_null()
        {
            let pct_s = b"%s\0";
            libc::printf(pct_s.as_ptr() as *const c_char, buffer.as_ptr());
        }

        if libc::ferror(fp) != 0 {
            // goto cleanup
            let err_fmt = b"Error: opening or processing file %s\n\0";
            libc::fprintf(stderr, err_fmt.as_ptr() as *const c_char, filename);
            libc::fclose(fp);
            return ptr::null_mut();
        }

        fp
    }
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
