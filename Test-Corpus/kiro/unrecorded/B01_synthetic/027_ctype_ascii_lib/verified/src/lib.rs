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
    unsafe {
        let locale = CString::new("C").unwrap();
        setlocale(LC_ALL, locale.as_ptr());

        let ci = c as c_int;

        let fmt_d = |label: &str, val: c_int| {
            let s = CString::new(format!("{}: %d\n", label)).unwrap();
            printf(s.as_ptr(), val);
        };
        let fmt_c = |label: &str, val: c_int| {
            let s = CString::new(format!("{}: %c\n", label)).unwrap();
            printf(s.as_ptr(), val);
        };

        fmt_d("alphanumeric", isalnum(ci));
        fmt_d("alphabetic", isalpha(ci));
        fmt_d("lowercase", islower(ci));
        fmt_d("uppercase", isupper(ci));
        fmt_d("digit", isdigit(ci));
        fmt_d("hexadecimal", isxdigit(ci));
        fmt_d("control", iscntrl(ci));
        fmt_d("graphical", isgraph(ci));
        fmt_d("space", isspace(ci));
        fmt_d("blank", isblank(ci));
        fmt_d("printing", isprint(ci));
        fmt_d("punctuation", ispunct(ci));
        fmt_c("to lower", tolower(ci));
        fmt_c("to upper", toupper(ci));
    }
}
