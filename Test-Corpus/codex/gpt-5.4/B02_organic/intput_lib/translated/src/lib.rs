use std::ffi::{c_char, c_int};

#[derive(Clone, Copy)]
struct Entry {
    key: c_int,
    value: c_int,
}

const FILE_NAME: &[u8] = b"c_src/src/lib.c\0";
const FUNC_NAME: &[u8] = b"intput\0";
const ASSERT_953: &[u8] = b"hmget(intmap, 9) == num\0";
const ASSERT_954: &[u8] = b"hmget(intmap, 11) == 3\0";
const ASSERT_955: &[u8] = b"hmget(intmap, num) == 7\0";

unsafe extern "C" {
    fn __assert_fail(
        assertion: *const c_char,
        file: *const c_char,
        line: u32,
        function: *const c_char,
    ) -> !;
}

fn hmput(intmap: &mut Vec<Entry>, key: c_int, value: c_int) {
    for entry in intmap.iter_mut() {
        if entry.key == key {
            entry.value = value;
            return;
        }
    }

    intmap.push(Entry { key, value });
}

fn hmget(intmap: &[Entry], key: c_int) -> c_int {
    for entry in intmap {
        if entry.key == key {
            return entry.value;
        }
    }

    0
}

#[cold]
fn assert_fail(assertion: &'static [u8], line: u32) -> ! {
    unsafe {
        __assert_fail(
            assertion.as_ptr().cast(),
            FILE_NAME.as_ptr().cast(),
            line,
            FUNC_NAME.as_ptr().cast(),
        )
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn intput(num: c_int) {
    let mut intmap = Vec::new();

    hmput(&mut intmap, num, 7);
    hmput(&mut intmap, 11, 3);
    hmput(&mut intmap, 9, num);

    if hmget(&intmap, 9) != num {
        assert_fail(ASSERT_953, 953);
    }
    if hmget(&intmap, 11) != 3 {
        assert_fail(ASSERT_954, 954);
    }
    if hmget(&intmap, num) != 7 {
        assert_fail(ASSERT_955, 955);
    }
}
