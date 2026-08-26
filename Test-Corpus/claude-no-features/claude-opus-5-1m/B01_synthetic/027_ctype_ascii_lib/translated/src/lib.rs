use std::ffi::CString;
use std::os::raw::{c_char, c_int};

extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn printf(fmt: *const c_char, ...) -> c_int;
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

        let ci: c_int = c as c_int;

        let f_alphanumeric = CString::new("alphanumeric: %d\n").unwrap();
        let f_alphabetic = CString::new("alphabetic: %d\n").unwrap();
        let f_lowercase = CString::new("lowercase: %d\n").unwrap();
        let f_uppercase = CString::new("uppercase: %d\n").unwrap();
        let f_digit = CString::new("digit: %d\n").unwrap();
        let f_hexadecimal = CString::new("hexadecimal: %d\n").unwrap();
        let f_control = CString::new("control: %d\n").unwrap();
        let f_graphical = CString::new("graphical: %d\n").unwrap();
        let f_space = CString::new("space: %d\n").unwrap();
        let f_blank = CString::new("blank: %d\n").unwrap();
        let f_printing = CString::new("printing: %d\n").unwrap();
        let f_punctuation = CString::new("punctuation: %d\n").unwrap();
        let f_to_lower = CString::new("to lower: %c\n").unwrap();
        let f_to_upper = CString::new("to upper: %c\n").unwrap();

        printf(f_alphanumeric.as_ptr(), isalnum(ci));
        printf(f_alphabetic.as_ptr(), isalpha(ci));
        printf(f_lowercase.as_ptr(), islower(ci));
        printf(f_uppercase.as_ptr(), isupper(ci));
        printf(f_digit.as_ptr(), isdigit(ci));
        printf(f_hexadecimal.as_ptr(), isxdigit(ci));
        printf(f_control.as_ptr(), iscntrl(ci));
        printf(f_graphical.as_ptr(), isgraph(ci));
        printf(f_space.as_ptr(), isspace(ci));
        printf(f_blank.as_ptr(), isblank(ci));
        printf(f_printing.as_ptr(), isprint(ci));
        printf(f_punctuation.as_ptr(), ispunct(ci));
        printf(f_to_lower.as_ptr(), tolower(ci));
        printf(f_to_upper.as_ptr(), toupper(ci));
    }
}
