// Translation of c_src/src/main.c -- see src/echo.rs for the translated
// `main()` body and a discussion of the C stdio semantics it reproduces.
//
//     /* interactive echo; ignores arguments, copies stdin to stdout */
//     int main() {
//         char text[128];
//
//         while (fgets(text, 128, stdin)) {
//             fputs(text, stdout);
//         }
//         return 0;
//     }

mod echo;

const SIGPIPE: i32 = 13;
const SIG_DFL: usize = 0;

extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
}

fn main() {
    // A C program starts with the default disposition for SIGPIPE, so the C
    // binary is killed by SIGPIPE (status 128+13) once the reader of its
    // stdout goes away.  The Rust runtime instead sets SIGPIPE to SIG_IGN
    // before calling `main`, which would turn that into a plain exit status of
    // 0; restore the C behaviour.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }

    // `main` ignores argc/argv, exactly like the C `int main()`.
    std::process::exit(echo::run());
}
