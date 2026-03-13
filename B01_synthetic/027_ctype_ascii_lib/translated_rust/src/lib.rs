#![no_builtins]

use std::ffi::{c_char, c_int, CString};

extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn printf(format: *const c_char, ...) -> c_int;
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
}

const LC_ALL: c_int = 6;

#[unsafe(no_mangle)]
pub extern "C" fn driver(c: c_char) {
    let ci = c as c_int;
    let locale = CString::new("C").unwrap();
    let fmt_d = CString::new("%s: %d\n").unwrap();
    let fmt_c = CString::new("to %s: %c\n").unwrap();

    unsafe {
        setlocale(LC_ALL, locale.as_ptr());

        let p = |label: &str, val: c_int| {
            let l = CString::new(label).unwrap();
            printf(fmt_d.as_ptr(), l.as_ptr(), val);
        };

        p("alphanumeric", isalnum(ci));
        p("alphabetic", isalpha(ci));
        p("lowercase", islower(ci));
        p("uppercase", isupper(ci));
        p("digit", isdigit(ci));
        p("hexadecimal", isxdigit(ci));
        p("control", iscntrl(ci));
        p("graphical", isgraph(ci));
        p("space", isspace(ci));
        p("blank", isblank(ci));
        p("printing", isprint(ci));
        p("punctuation", ispunct(ci));

        let l1 = CString::new("lower").unwrap();
        printf(fmt_c.as_ptr(), l1.as_ptr(), tolower(ci));
        let l2 = CString::new("upper").unwrap();
        printf(fmt_c.as_ptr(), l2.as_ptr(), toupper(ci));
    }
}
