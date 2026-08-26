use libc::{c_char, c_int};

fn driver(c: c_char) {
    let c_int_value = c as c_int;

    unsafe {
        libc::setlocale(libc::LC_ALL, c"C".as_ptr());

        libc::printf(c"alphanumeric: %d\n".as_ptr(), libc::isalnum(c_int_value));
        libc::printf(c"alphabetic: %d\n".as_ptr(), libc::isalpha(c_int_value));
        libc::printf(c"lowercase: %d\n".as_ptr(), libc::islower(c_int_value));
        libc::printf(c"uppercase: %d\n".as_ptr(), libc::isupper(c_int_value));
        libc::printf(c"digit: %d\n".as_ptr(), libc::isdigit(c_int_value));
        libc::printf(c"hexadecimal: %d\n".as_ptr(), libc::isxdigit(c_int_value));
        libc::printf(c"control: %d\n".as_ptr(), libc::iscntrl(c_int_value));
        libc::printf(c"graphical: %d\n".as_ptr(), libc::isgraph(c_int_value));
        libc::printf(c"space: %d\n".as_ptr(), libc::isspace(c_int_value));
        libc::printf(c"blank: %d\n".as_ptr(), libc::isblank(c_int_value));
        libc::printf(c"printing: %d\n".as_ptr(), libc::isprint(c_int_value));
        libc::printf(c"punctuation: %d\n".as_ptr(), libc::ispunct(c_int_value));
        libc::printf(c"to lower: %c\n".as_ptr(), libc::tolower(c_int_value));
        libc::printf(c"to upper: %c\n".as_ptr(), libc::toupper(c_int_value));
    }
}

fn main() {
    let c = unsafe { libc::getchar() as c_char };
    driver(c);
}
