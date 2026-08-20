// C-ABI mirror of `c_src/src/luggage.c`.
//
// This target exists so that the translated logic can be loaded and called
// through a shared library exactly the way an external C caller would call the
// original translation unit (which the differential tests build as
// `libluggage.so` with `gcc -shared -fPIC -Dmain=luggage_main`, i.e. without
// touching `c_src/`).
//
// Every exported symbol has the same name and the same C signature as in the C
// source, and each one delegates to `luggage_core` — the very same code the
// `driver` executable runs.

pub mod luggage_core;

use luggage_core::{Arena, RoutingDirective};
use std::ffi::c_char;
use std::ffi::c_int;
use std::io::Write;

pub const LUGGAGE_ID_LENGTH: usize = 8;
pub const FLIGHT_ID_LENGTH: usize = 6;
pub const AIRPORT_CODE_LENGTH: usize = 3;
pub const COMMENTS_LENGTH: usize = 80;

/// Byte-for-byte mirror of the C `RoutingDirective` (size 120, offsets
/// 0/4/13/20/24/28/112 — verified against `offsetof` on the C compiler).
#[repr(C)]
pub struct CRoutingDirective {
    pub time_stamp: u32,
    pub luggage_id: [u8; LUGGAGE_ID_LENGTH + 1],
    pub flight_id: [u8; FLIGHT_ID_LENGTH + 1],
    pub departure: [u8; AIRPORT_CODE_LENGTH + 1],
    pub arrival: [u8; AIRPORT_CODE_LENGTH + 1],
    pub comments: [u8; COMMENTS_LENGTH + 1],
    pub next_directive: *mut CRoutingDirective,
}

/// Reads a NUL terminated C string as bytes (without the NUL).
unsafe fn cstr_bytes(p: *const c_char) -> Vec<u8> {
    if p.is_null() {
        // The C code dereferences unconditionally; a NULL here is UB in C.
        // Returning an empty string keeps the Rust side memory-safe.
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut i = 0isize;
    loop {
        let b = *p.offset(i) as u8;
        if b == 0 {
            break;
        }
        out.push(b);
        i += 1;
    }
    out
}

/// Turns a C linked list into the arena representation used by the
/// translation.  `nodes[0]` stays the dummy head that the C `main` keeps on the
/// stack; the C chain occupies indices `1..=n`.  Returns the arena, the C
/// pointer for every arena index (`ptrs[i - 1]`) and the arena index of the
/// first chain element.
unsafe fn chain_to_arena(
    head: *mut CRoutingDirective,
) -> (Arena, Vec<*mut CRoutingDirective>, Option<usize>) {
    let mut arena = Arena::new();
    let mut ptrs: Vec<*mut CRoutingDirective> = Vec::new();
    let mut p = head;
    while !p.is_null() {
        let n = &*p;
        arena.nodes.push(RoutingDirective {
            time_stamp: n.time_stamp,
            luggage_id: n.luggage_id.to_vec(),
            flight_id: n.flight_id.to_vec(),
            departure: n.departure.to_vec(),
            arrival: n.arrival.to_vec(),
            comments: n.comments.to_vec(),
            next_directive: None,
        });
        ptrs.push(p);
        p = n.next_directive;
    }
    // Link the arena nodes the same way the C chain is linked.
    let count = ptrs.len();
    for i in 1..=count {
        arena.nodes[i].next_directive = if i < count { Some(i + 1) } else { None };
    }
    let first = if count == 0 { None } else { Some(1) };
    (arena, ptrs, first)
}

/// `void addRoutingDirectiveToList(RoutingDirective *previous_directive,
///                                 RoutingDirective *new_directive)`
#[no_mangle]
pub unsafe extern "C" fn addRoutingDirectiveToList(
    previous_directive: *mut CRoutingDirective,
    new_directive: *mut CRoutingDirective,
) {
    if previous_directive.is_null() || new_directive.is_null() {
        // C would dereference NULL here (UB); nothing sensible to mirror.
        return;
    }
    // Mirror the C list (starting at `previous_directive`) into the arena,
    // append the new node, run the translated insertion routine and write the
    // resulting order back into the C `next_directive` pointers.
    let (mut arena, ptrs, _first) = chain_to_arena(previous_directive);
    let n = &*new_directive;
    arena.nodes.push(RoutingDirective {
        time_stamp: n.time_stamp,
        luggage_id: n.luggage_id.to_vec(),
        flight_id: n.flight_id.to_vec(),
        departure: n.departure.to_vec(),
        arrival: n.arrival.to_vec(),
        comments: n.comments.to_vec(),
        next_directive: None,
    });
    let new_idx = arena.nodes.len() - 1;

    // `previous_directive` is arena index 1 (it is the head of the mirrored
    // chain).
    luggage_core::add_routing_directive_to_list(&mut arena, 1, new_idx);

    let ptr_of = |idx: usize| -> *mut CRoutingDirective {
        if idx == new_idx {
            new_directive
        } else {
            ptrs[idx - 1]
        }
    };
    let mut idx = 1usize;
    loop {
        let next = arena.nodes[idx].next_directive;
        let cptr = ptr_of(idx);
        (*cptr).next_directive = match next {
            Some(n) => ptr_of(n),
            None => std::ptr::null_mut(),
        };
        match next {
            Some(n) => idx = n,
            None => break,
        }
    }
}

/// `int supersedes(RoutingDirective *directive, char *luggage_id, char *departure)`
#[no_mangle]
pub unsafe extern "C" fn supersedes(
    directive: *mut CRoutingDirective,
    luggage_id: *mut c_char,
    departure: *mut c_char,
) -> c_int {
    let (arena, _ptrs, first) = chain_to_arena(directive);
    let lug = cstr_bytes(luggage_id);
    let dep = cstr_bytes(departure);
    if luggage_core::supersedes(&arena, first, &lug, &dep) {
        1
    } else {
        0
    }
}

/// `int superseded(RoutingDirective *directive)`
#[no_mangle]
pub unsafe extern "C" fn superseded(directive: *mut CRoutingDirective) -> c_int {
    if directive.is_null() {
        // C dereferences unconditionally (UB).
        return 0;
    }
    let (arena, _ptrs, _first) = chain_to_arena(directive);
    if luggage_core::superseded(&arena, 1) {
        1
    } else {
        0
    }
}

/// `int matches(char *expected, char *actual)`
#[no_mangle]
pub unsafe extern "C" fn matches(expected: *mut c_char, actual: *mut c_char) -> c_int {
    let e = cstr_bytes(expected);
    let a = cstr_bytes(actual);
    if luggage_core::matches(&e, &a) {
        1
    } else {
        0
    }
}

/// Unbuffered writer for file descriptor 1, so that the shared library writes
/// straight to `stdout` like C's `printf` does after a flush.
struct Fd1;

impl Write for Fd1 {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        use std::os::unix::io::FromRawFd;
        let mut f = std::mem::ManuallyDrop::new(unsafe { std::fs::File::from_raw_fd(1) });
        f.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// `void printMatchingDirectives(RoutingDirective *first_directive,
///                               char *expected_luggage_id,
///                               char *expected_flight_id,
///                               char *expected_departure,
///                               char *expected_arrival)`
#[no_mangle]
pub unsafe extern "C" fn printMatchingDirectives(
    first_directive: *mut CRoutingDirective,
    expected_luggage_id: *mut c_char,
    expected_flight_id: *mut c_char,
    expected_departure: *mut c_char,
    expected_arrival: *mut c_char,
) {
    let (arena, _ptrs, first) = chain_to_arena(first_directive);
    let mut out = Fd1;
    luggage_core::print_matching_directives(
        &mut out,
        &arena,
        first,
        &cstr_bytes(expected_luggage_id),
        &cstr_bytes(expected_flight_id),
        &cstr_bytes(expected_departure),
        &cstr_bytes(expected_arrival),
    );
    let _ = out.flush();
}

/// `int main(int argc, char *argv[])` of the C program (exported as
/// `luggage_main`, matching the `-Dmain=luggage_main` build of the C source).
/// Never returns: it ends with `exit()` just like the C code.
#[no_mangle]
pub unsafe extern "C" fn luggage_main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    let mut args: Vec<Vec<u8>> = Vec::new();
    if !argv.is_null() {
        for i in 0..argc.max(0) {
            args.push(cstr_bytes(*argv.offset(i as isize)));
        }
    }
    luggage_core::luggage_main_impl(&args);
}
