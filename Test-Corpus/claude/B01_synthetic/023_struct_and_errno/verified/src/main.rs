// `driver` executable — the translation of c_src/src/main.c.
//
// The translated code lives in `imp.rs`, which is included directly (rather
// than through the library target) because the library target is a `cdylib`
// that also exports a C-ABI `main` symbol; linking it into this binary would
// collide with the binary's own `main`.

#[path = "imp.rs"]
mod imp;

// ---------------------------------------------------------------------------
// SIGPIPE fidelity
// ---------------------------------------------------------------------------
// The C program never touches signal dispositions, so it runs with whatever it
// inherited: with the usual default disposition a `printf` to a pipe whose
// reader is gone kills it with SIGPIPE (exit status 141 / signal 13).
// Rust's runtime, however, sets SIGPIPE to SIG_IGN before calling `main`, which
// would make this translation exit 0 where the C dies.
//
// An ELF constructor runs before the Rust runtime initialises, so it can record
// the disposition the process actually inherited; `main` then restores it as its
// very first action. That reproduces the C behaviour in both directions: the
// program dies on SIGPIPE when the parent's disposition is the default, and
// keeps ignoring it when the parent was ignoring it.

const SIGPIPE: i32 = 13;
const SIG_DFL: usize = 0;
const SIG_ERR: usize = usize::MAX;

extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
}

static mut INHERITED_SIGPIPE: usize = SIG_DFL;

extern "C" fn record_inherited_sigpipe() {
    unsafe {
        // `signal` is the portable way to *read* the current disposition
        // without pulling in `struct sigaction`: query by installing SIG_DFL
        // and immediately putting the old value back.
        let old = signal(SIGPIPE, SIG_DFL);
        if old != SIG_ERR {
            signal(SIGPIPE, old);
            INHERITED_SIGPIPE = old;
        }
    }
}

#[used]
#[link_section = ".init_array"]
static RECORD_INHERITED_SIGPIPE: extern "C" fn() = record_inherited_sigpipe;

fn main() {
    unsafe {
        signal(SIGPIPE, INHERITED_SIGPIPE);
    }
    imp::program_main();
}
