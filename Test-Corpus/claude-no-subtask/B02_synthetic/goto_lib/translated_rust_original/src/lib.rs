use std::ffi::c_char;
use std::ffi::c_int;
use std::ptr;

fn forward_goto_example(x: c_int) -> c_int {
    if x < 0 {
        // error label
        unsafe {
            let msg = b"Error: negative input\n\0";
            libc::fprintf(
                libc_stderr(),
                msg.as_ptr() as *const c_char,
            );
        }
        return -1;
    }

    unsafe {
        let fmt = b"Processing: %d\n\0";
        libc::printf(fmt.as_ptr() as *const c_char, x);
    }
    x * 2
}

fn libc_stderr() -> *mut libc::FILE {
    extern "C" {
        // On glibc, stderr is a symbol of type FILE*. We declare it here.
        static mut stderr: *mut libc::FILE;
    }
    unsafe { stderr }
}

unsafe fn open_with_cleanup(filename: *const c_char) -> *mut libc::FILE {
    let mode = b"r\0";
    let fp = libc::fopen(filename, mode.as_ptr() as *const c_char);
    if fp.is_null() {
        // cleanup label
        let fmt = b"Error: opening or processing file %s\n\0";
        libc::fprintf(libc_stderr(), fmt.as_ptr() as *const c_char, filename);
        // if(fp) fclose(fp); -- fp is null here, so skip
        return ptr::null_mut();
    }

    let mut buffer = [0u8; 100];
    while !libc::fgets(
        buffer.as_mut_ptr() as *mut c_char,
        buffer.len() as c_int,
        fp,
    )
    .is_null()
    {
        let fmt = b"%s\0";
        libc::printf(fmt.as_ptr() as *const c_char, buffer.as_ptr() as *const c_char);
    }

    if libc::ferror(fp) != 0 {
        // cleanup label
        let fmt = b"Error: opening or processing file %s\n\0";
        libc::fprintf(libc_stderr(), fmt.as_ptr() as *const c_char, filename);
        libc::fclose(fp);
        return ptr::null_mut();
    }

    fp
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(num: c_int, filename: *const c_char) -> c_int {
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
