//! Phase D — symbol parity between the C shared object and the Rust `cdylib`.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use common::*;

fn nm(path: &Path, flag: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", flag])
        .arg(path)
        .output()
        .expect("nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

/// Symbols glibc / libgcc / the Rust runtime are expected to import.
fn is_runtime_import(sym: &str) -> bool {
    let base = sym.split('@').next().unwrap_or(sym);
    base.starts_with("_ITM_")
        || base.starts_with("_Unwind_")
        || base.starts_with("__")
        || matches!(
            base,
            "abort"
                | "bcmp"
                | "calloc"
                | "close"
                | "dl_iterate_phdr"
                | "exit"
                | "fclose"
                | "fgets"
                | "fopen"
                | "fprintf"
                | "free"
                | "getchar"
                | "getcwd"
                | "getenv"
                | "gettid"
                | "malloc"
                | "memcpy"
                | "memmove"
                | "memset"
                | "mmap64"
                | "munmap"
                | "open64"
                | "lseek64"
                | "posix_memalign"
                | "printf"
                | "pthread_key_create"
                | "pthread_key_delete"
                | "pthread_setspecific"
                | "pthread_getspecific"
                | "pthread_mutex_lock"
                | "pthread_mutex_trylock"
                | "pthread_mutex_unlock"
                | "pthread_self"
                | "read"
                | "readlink"
                | "realloc"
                | "realpath"
                | "sigaction"
                | "sigaltstack"
                | "signal"
                | "stat64"
                | "statx"
                | "fstat64"
                | "stderr"
                | "stdin"
                | "stdout"
                | "strcspn"
                | "strlen"
                | "strncpy"
                | "strcpy"
                | "sysconf"
                | "syscall"
                | "write"
                | "writev"
                | "poll"
                | "dlopen"
                | "dlsym"
                | "dlclose"
                | "dlerror"
                | "dladdr"
                | "getrandom"
                | "pipe2"
                | "fcntl"
                | "mprotect"
                | "pthread_attr_init"
                | "pthread_attr_destroy"
                | "pthread_attr_setstacksize"
                | "pthread_create"
                | "pthread_detach"
                | "pthread_join"
                | "pthread_rwlock_rdlock"
                | "pthread_rwlock_unlock"
                | "pthread_rwlock_wrlock"
                | "nanosleep"
                | "sched_yield"
                | "environ"
                | "unlink"
                | "getpid"
                | "isatty"
                | "fdopen"
                | "fflush"
                | "fscanf"
                | "scanf"
                | "sscanf"
                | "fwrite"
                | "fputs"
                | "puts"
                | "putchar"
                | "putc"
                | "fputc"
                | "vfprintf"
                | "vprintf"
        )
}

#[test]
fn symbol_parity() {
    let c_path = c_lib_path();
    let r_path = rust_lib_path();

    let c_defined = nm(&c_path, "--defined-only");
    let r_defined = nm(&r_path, "--defined-only");

    let missing: Vec<&String> = c_defined.difference(&r_defined).collect();
    let extra: Vec<&String> = r_defined.difference(&c_defined).collect();

    eprintln!(
        "C exports {} symbols, Rust exports {}",
        c_defined.len(),
        r_defined.len()
    );
    assert!(
        missing.is_empty(),
        "the Rust shared object is missing {} symbol(s) exported by the C shared object: {:?}",
        missing.len(),
        missing
    );
    assert!(
        extra.is_empty(),
        "the Rust shared object exports {} symbol(s) the C one does not: {:?}",
        extra.len(),
        extra
    );

    // Every exported symbol must really be resolvable through dlsym (this is
    // what an external caller does).
    let apis = load_apis();
    assert_eq!(apis.c.which, "C");
    assert_eq!(apis.rust.which, "RUST");

    // No unexpected undefined symbol on either side.
    for (name, path) in [("C", &c_path), ("RUST", &r_path)] {
        let undef = nm(path, "--undefined-only");
        let unexpected: Vec<&String> = undef
            .iter()
            .filter(|s| !is_runtime_import(s))
            .filter(|s| !c_defined.contains(&s.split('@').next().unwrap().to_string()))
            .collect();
        assert!(
            unexpected.is_empty(),
            "{} has unexpected undefined symbols: {:?}",
            name,
            unexpected
        );
    }
}

/// The struct layouts the tests (and the Rust translation) use must be the ones
/// the C compiler produces.
#[test]
fn struct_layout() {
    use std::mem::{align_of, size_of};
    assert_eq!(size_of::<ShapeT>(), 2444);
    assert_eq!(align_of::<ShapeT>(), 4);
    assert_eq!(size_of::<SceneT>(), 472);
    assert_eq!(align_of::<SceneT>(), 8);

    let s = ShapeT {
        type_: 0,
        name: [0; MAX_SHAPE_NAME],
        art: [[0; MAX_SHAPE_WIDTH]; MAX_SHAPE_HEIGHT],
        width: 0,
        height: 0,
    };
    let base = &s as *const ShapeT as usize;
    assert_eq!(&s.type_ as *const _ as usize - base, 0);
    assert_eq!(&s.name as *const _ as usize - base, 4);
    assert_eq!(&s.art as *const _ as usize - base, 36);
    assert_eq!(&s.width as *const _ as usize - base, 2436);
    assert_eq!(&s.height as *const _ as usize - base, 2440);

    let sc = SceneT {
        name: [0; MAX_SCENE_NAME],
        shapes: [std::ptr::null_mut(); MAX_SHAPES_IN_SCENE],
        shape_count: 0,
    };
    let base = &sc as *const SceneT as usize;
    assert_eq!(&sc.name as *const _ as usize - base, 0);
    assert_eq!(&sc.shapes as *const _ as usize - base, 64);
    assert_eq!(&sc.shape_count as *const _ as usize - base, 464);
}
