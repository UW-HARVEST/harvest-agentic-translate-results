use std::ffi::CString;
use std::os::raw::{c_char, c_int};

extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn isalnum(c: c_int) -> c_int;
    fn isalpha(c: c_int) -> c_int;
    fn islower(c: c_int) -> c_int;
    fn isupper(c: c_int) -> c_int;
    fn isdigit(c: c_int) -> c_int;
    fn isxdigit(c: c_int) -> c_int;
    fn iscntrl(c: c_int) -> c_int;
    fn isgraph(c: c_int) -> c_int;
    fn isspace(c: c_int) -> c_int;
    fn isblank(c: c_int) -> c_int;
    fn isprint(c: c_int) -> c_int;
    fn ispunct(c: c_int) -> c_int;
    fn tolower(c: c_int) -> c_int;
    fn toupper(c: c_int) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn getchar() -> c_int;
}

// On glibc Linux, LC_ALL = 6. Use libc binding via direct constant.
const LC_ALL: c_int = 6;

#[unsafe(no_mangle)]
pub extern "C" fn driver(c: c_char) {
    unsafe {
        let locale = CString::new("C").unwrap();
        setlocale(LC_ALL, locale.as_ptr());

        let ci = c as c_int;

        let f_int = CString::new("%s%d\n").unwrap();
        let f_char = CString::new("%s%c\n").unwrap();

        let l_alnum = CString::new("alphanumeric: ").unwrap();
        let l_alpha = CString::new("alphabetic: ").unwrap();
        let l_lower = CString::new("lowercase: ").unwrap();
        let l_upper = CString::new("uppercase: ").unwrap();
        let l_digit = CString::new("digit: ").unwrap();
        let l_hex = CString::new("hexadecimal: ").unwrap();
        let l_ctrl = CString::new("control: ").unwrap();
        let l_graph = CString::new("graphical: ").unwrap();
        let l_space = CString::new("space: ").unwrap();
        let l_blank = CString::new("blank: ").unwrap();
        let l_print = CString::new("printing: ").unwrap();
        let l_punct = CString::new("punctuation: ").unwrap();
        let l_tolow = CString::new("to lower: ").unwrap();
        let l_toup = CString::new("to upper: ").unwrap();

        printf(f_int.as_ptr(), l_alnum.as_ptr(), isalnum(ci));
        printf(f_int.as_ptr(), l_alpha.as_ptr(), isalpha(ci));
        printf(f_int.as_ptr(), l_lower.as_ptr(), islower(ci));
        printf(f_int.as_ptr(), l_upper.as_ptr(), isupper(ci));
        printf(f_int.as_ptr(), l_digit.as_ptr(), isdigit(ci));
        printf(f_int.as_ptr(), l_hex.as_ptr(), isxdigit(ci));
        printf(f_int.as_ptr(), l_ctrl.as_ptr(), iscntrl(ci));
        printf(f_int.as_ptr(), l_graph.as_ptr(), isgraph(ci));
        printf(f_int.as_ptr(), l_space.as_ptr(), isspace(ci));
        printf(f_int.as_ptr(), l_blank.as_ptr(), isblank(ci));
        printf(f_int.as_ptr(), l_print.as_ptr(), isprint(ci));
        printf(f_int.as_ptr(), l_punct.as_ptr(), ispunct(ci));
        printf(f_char.as_ptr(), l_tolow.as_ptr(), tolower(ci));
        printf(f_char.as_ptr(), l_toup.as_ptr(), toupper(ci));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> c_int {
    unsafe {
        let c = getchar();
        driver(c as c_char);
    }
    0
}
