//! Phase B — valid-path differential tests for the logger entry points
//! (`CONFIGS.md` rows 1-14).
//!
//! Every call goes through the exported symbols of both `.so`s, loaded with
//! `libloading`; the C result is the reference.

mod common;

use common::{assert_same, cstring, Config, LogTarget, Rng};
use std::ffi::c_char;

const SEED: u64 = 0xC0FFEE_1234_5678;

/// How many randomized inputs each property-style row uses.
const N: usize = 64;

// ---------------------------------------------------------------------------
// Rows 1-5: LOG_FILE targets
// ---------------------------------------------------------------------------

#[test]
fn cfg01_log_default_path() {
    // $LOG_FILE unset -> the C opens the literal "default.log" in the cwd.
    let mut rng = Rng::new(SEED);
    for i in 0..N {
        let n = rng.below(80);
        let msg = cstring(&rng.cstr_body(n));
        assert_same(
            &format!("cfg01-{i}"),
            &Config::new().log(LogTarget::Unset),
            |api| unsafe {
                let r = (api.initialize_logger)();
                (api.log_info)(msg.as_ptr() as *const c_char);
                (api.finalize_logger)();
                r as i64
            },
        );
    }
}

#[test]
fn cfg02_log_relative_path() {
    let mut rng = Rng::new(SEED + 1);
    for i in 0..N {
        let n = rng.below(120);
        let msg = cstring(&rng.cstr_body(n));
        assert_same(
            &format!("cfg02-{i}"),
            &Config::new().log(LogTarget::Relative("sub_relative.log")),
            |api| unsafe {
                let r = (api.initialize_logger)();
                (api.log_warning)(msg.as_ptr() as *const c_char);
                (api.finalize_logger)();
                r as i64
            },
        );
    }
}

#[test]
fn cfg03_log_absolute_path() {
    let mut rng = Rng::new(SEED + 2);
    for i in 0..N {
        let n = rng.below(120);
        let msg = cstring(&rng.cstr_body(n));
        assert_same(
            &format!("cfg03-{i}"),
            &Config::new().log(LogTarget::Absolute("abs.log")),
            |api| unsafe {
                let r = (api.initialize_logger)();
                (api.log_error)(msg.as_ptr() as *const c_char);
                (api.finalize_logger)();
                r as i64
            },
        );
    }
}

#[test]
fn cfg04_log_appends() {
    // fopen(path, "a") must append to an existing file, not truncate it.
    let mut rng = Rng::new(SEED + 3);
    for i in 0..N {
        let n = rng.range(1, 60);
        let msg = cstring(&rng.cstr_body(n));
        let n = rng.range(1, 40);
        let preamble: Vec<u8> = rng.cstr_body(n);
        assert_same(
            &format!("cfg04-{i}"),
            &Config::new().log(LogTarget::Relative("append.log")),
            |api| unsafe {
                // The harness has chdir'ed into this side's scratch dir, so the
                // relative path is the very file $LOG_FILE names.
                let mut seed = preamble.clone();
                seed.push(b'\n');
                std::fs::write("append.log", &seed).unwrap();
                let r = (api.initialize_logger)();
                (api.log_info)(msg.as_ptr() as *const c_char);
                (api.finalize_logger)();
                r as i64
            },
        );
    }
}

#[test]
fn cfg05_log_devnull() {
    let mut rng = Rng::new(SEED + 4);
    for i in 0..16 {
        let n = rng.below(200);
        let msg = cstring(&rng.cstr_body(n));
        assert_same(
            &format!("cfg05-{i}"),
            &Config::new().log(LogTarget::Raw("/dev/null".into())),
            |api| unsafe {
                let r = (api.initialize_logger)();
                (api.log_info)(msg.as_ptr() as *const c_char);
                (api.log_warning)(msg.as_ptr() as *const c_char);
                (api.log_error)(msg.as_ptr() as *const c_char);
                (api.finalize_logger)();
                r as i64
            },
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 6-14: message shapes
// ---------------------------------------------------------------------------

/// ASCII-only body of `len` bytes.
fn ascii(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| rng.range(0x20, 0x7e) as u8).collect()
}

fn one_level_row(tag: &str, seed: u64, which: usize, body: impl Fn(&mut Rng) -> Vec<u8>) {
    let mut rng = Rng::new(seed);
    for i in 0..N {
        let msg = cstring(&body(&mut rng));
        assert_same(&format!("{tag}-{i}"), &Config::new(), |api| unsafe {
            let r = (api.initialize_logger)();
            let p = msg.as_ptr() as *const c_char;
            match which {
                0 => (api.log_info)(p),
                1 => (api.log_warning)(p),
                _ => (api.log_error)(p),
            }
            (api.finalize_logger)();
            r as i64
        });
    }
}

#[test]
fn cfg06_log_info_ascii() {
    one_level_row("cfg06", SEED + 5, 0, |r| {
        let n = r.below(200);
        ascii(r, n)
    });
}

#[test]
fn cfg07_log_warning_ascii() {
    one_level_row("cfg07", SEED + 6, 1, |r| {
        let n = r.below(200);
        ascii(r, n)
    });
}

#[test]
fn cfg08_log_error_ascii() {
    one_level_row("cfg08", SEED + 7, 2, |r| {
        let n = r.below(200);
        ascii(r, n)
    });
}

#[test]
fn cfg09_log_non_utf8() {
    // Bytes 0x80..0xFF: a translation that round-trips through `String` or
    // `str::from_utf8` mangles or panics on these.
    let mut rng = Rng::new(SEED + 8);
    for i in 0..N {
        let blen = rng.range(1, 250);
        let body: Vec<u8> = (0..blen)
            .map(|_| rng.range(0x80, 0xff) as u8)
            .collect();
        let msg = cstring(&body);
        assert_same(&format!("cfg09-{i}"), &Config::new(), |api| unsafe {
            let r = (api.initialize_logger)();
            let p = msg.as_ptr() as *const c_char;
            (api.log_info)(p);
            (api.log_warning)(p);
            (api.log_error)(p);
            (api.finalize_logger)();
            r as i64
        });
    }
}

#[test]
fn cfg10_log_format_specifiers() {
    // The C passes the message as a `%s` *argument*, so conversion specifiers
    // inside it must be emitted verbatim.
    let pieces: &[&[u8]] = &[
        b"%s", b"%d", b"%n", b"%p", b"%%", b"%1000000d", b"%.*s", b"%hhn", b"a%sb%dc",
    ];
    let mut rng = Rng::new(SEED + 9);
    for i in 0..N {
        let mut body = Vec::new();
        for _ in 0..rng.range(1, 10) {
            body.extend_from_slice(pieces[rng.below(pieces.len())]);
        }
        let msg = cstring(&body);
        assert_same(&format!("cfg10-{i}"), &Config::new(), |api| unsafe {
            let r = (api.initialize_logger)();
            let p = msg.as_ptr() as *const c_char;
            (api.log_info)(p);
            (api.log_warning)(p);
            (api.log_error)(p);
            (api.finalize_logger)();
            r as i64
        });
    }
}

#[test]
fn cfg11_log_short() {
    let mut rng = Rng::new(SEED + 10);
    for i in 0..N {
        let len = if i % 2 == 0 { 0 } else { 1 };
        let msg = cstring(&rng.cstr_body(len));
        assert_same(&format!("cfg11-{i}"), &Config::new(), |api| unsafe {
            let r = (api.initialize_logger)();
            let p = msg.as_ptr() as *const c_char;
            (api.log_info)(p);
            (api.log_warning)(p);
            (api.log_error)(p);
            (api.finalize_logger)();
            r as i64
        });
    }
}

#[test]
fn cfg12_log_long() {
    // Messages far bigger than BUFSIZ (8192 in glibc) so the stream flushes
    // mid-message; the split must land in the same place in both builds.
    let mut rng = Rng::new(SEED + 11);
    for (i, len) in [1, 4095, 4096, 4097, 8190, 8191, 8192, 8193, 20000]
        .into_iter()
        .enumerate()
    {
        let body = rng.cstr_body(len);
        let msg = cstring(&body);
        assert_same(&format!("cfg12-{i}"), &Config::new(), |api| unsafe {
            let r = (api.initialize_logger)();
            let p = msg.as_ptr() as *const c_char;
            (api.log_info)(p);
            (api.log_warning)(p);
            (api.log_error)(p);
            (api.finalize_logger)();
            r as i64
        });
    }
}

#[test]
fn cfg13_log_null_message() {
    // glibc's printf family prints the literal text "(null)" for a NULL `%s`.
    assert_same("cfg13", &Config::new(), |api| unsafe {
        let r = (api.initialize_logger)();
        (api.log_info)(std::ptr::null());
        (api.log_warning)(std::ptr::null());
        (api.log_error)(std::ptr::null());
        (api.finalize_logger)();
        r as i64
    });
}

#[test]
fn cfg14_log_interleaved() {
    let mut rng = Rng::new(SEED + 12);
    for i in 0..N {
        let mut script: Vec<(usize, Vec<u8>)> = Vec::new();
        for _ in 0..rng.range(1, 25) {
            let n = rng.below(70);
            script.push((rng.below(3), cstring(&rng.cstr_body(n))));
        }
        assert_same(&format!("cfg14-{i}"), &Config::new(), |api| unsafe {
            let r = (api.initialize_logger)();
            for (which, msg) in &script {
                let p = msg.as_ptr() as *const c_char;
                match which {
                    0 => (api.log_info)(p),
                    1 => (api.log_warning)(p),
                    _ => (api.log_error)(p),
                }
            }
            (api.finalize_logger)();
            r as i64
        });
    }
}
