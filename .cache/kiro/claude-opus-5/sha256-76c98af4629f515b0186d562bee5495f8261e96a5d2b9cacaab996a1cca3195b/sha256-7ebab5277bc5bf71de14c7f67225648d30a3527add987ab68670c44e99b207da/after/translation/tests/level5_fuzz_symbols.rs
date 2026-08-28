//! Level 5: exported-symbol parity between the two shared objects, and a
//! randomised differential fuzz of the whole parsing stack.

mod common;

use common::*;
use std::io::{Read, Seek, SeekFrom};
use std::os::raw::c_int;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Symbol parity
// ---------------------------------------------------------------------------

/// `nm -D --defined-only <so>` reduced to the set of exported names.
fn exported_symbols(so: &PathBuf) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("nm not available");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut names: Vec<String> = text
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Only code/data symbols a caller could bind to; skip the linker's
            // own bookkeeping entries.
            if matches!(kind, "T" | "t" | "D" | "B" | "R" | "W" | "V" | "G" | "S" | "i")
                && !name.starts_with("_ITM_")
                && !name.starts_with("__gmon")
                && name != "_init"
                && name != "_fini"
                && name != "__bss_start"
                && name != "_edata"
                && name != "_end"
            {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

#[test]
fn rust_exports_every_c_symbol() {
    let (c_so, rs_so) = so_paths();
    let c_syms = exported_symbols(&c_so);
    let rs_syms = exported_symbols(&rs_so);

    assert!(!c_syms.is_empty(), "nm reported no symbols for the C library");

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rs_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing symbols exported by the C .so: {missing:?}\n\
         C:    {c_syms:?}\nRust: {rs_syms:?}"
    );

    // The documented public surface must be present in both.
    for want in [
        "driver",
        "Init_FileQueue",
        "Read_FileMon",
        "GetAlertData",
        "FreeAlertData",
        "merror",
        "os_calloc",
        "os_realloc",
        "os_strdup",
    ] {
        assert!(c_syms.iter().any(|s| s == want), "C lacks {want}");
        assert!(rs_syms.iter().any(|s| s == want), "Rust lacks {want}");
    }

    // Symbols that are `static` in C must not leak out of the Rust build.
    for internal in ["file_sleep", "GetFile_Queue", "Handle_Queue", "s_month"] {
        assert!(
            !c_syms.iter().any(|s| s == internal),
            "unexpected: C exports {internal}"
        );
        assert!(
            !rs_syms.iter().any(|s| s == internal),
            "Rust must not export the internal helper {internal}"
        );
    }
}

// ---------------------------------------------------------------------------
// Randomised differential fuzz
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*, deterministic across platforms.
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

/// Builds a pseudo-random alert log.
///
/// Two inputs are deliberately excluded because the C code has undefined
/// behaviour on them and no translation could match it reliably:
///   * a line that is exactly `Integrity checksum changed for: '` (the
///     `filename[strlen(filename) - 1]` store then writes before the block), and
///   * a final `** Alert` line with no trailing newline (`str + 9` then reads
///     past the NUL into uninitialised stack memory).
fn random_log(rng: &mut Rng) -> Vec<u8> {
    const HEADERS: &[&str] = &[
        "** Alert 1755787624.1234: mail - syscheck,pci_dss_11.5,",
        "** Alert 1.1: mail - authentication_failed,",
        "** Alert 42.7: - syscheck,",
        "** Alert 99999.0: mail active-response - group_a,group_b,",
        "** Alert nocolon here",
        "** Alert 5:nospace",
        "** Alert 7: mail",
        "** Alert : mail - g,",
        "** Alert 8: mail -    syscheck   ,",
        "** Alert",
        "**Alert 3: mail - g,",
        "** alert 3: mail - g,",
    ];
    const DATES: &[&str] = &[
        "2025 Aug 21 13:27:04 (agent-01) 10.0.0.1->syscheck",
        "2006 Apr 13 16:15:17 /var/log/auth.log",
        "no colon at all in this line",
        "2025Aug21T13:27:04",
        "abc:",
        "",
        "x: y",
    ];
    const BODIES: &[&str] = &[
        "Rule: 550 (level 7) -> 'Integrity checksum changed.'",
        "Rule: 5710 (level 5) -> 'Attempt to login using a non-existent user'",
        "Rule: 550",
        "Rule: 550 x",
        "Rule: 550 (level 7)",
        "Rule: 550 (level 7) -> 'unterminated",
        "Rule: 550 (level 7) -> ''",
        "Rule: abc (level xyz) -> 'c'",
        "Rule: ",
        "Src IP: 192.168.1.10",
        "Src IP: ",
        "Src Port: 4242",
        "Src Port: ",
        "Src Port: junk",
        "Src Port: -7",
        "Dst IP: 10.1.2.3",
        "Dst IP: ",
        "Dst Port: 80",
        "Dst Port: 99999999999",
        "User: root",
        "User: ",
        "Integrity checksum changed for: '/etc/passwd'",
        "Integrity checksum changed for: 'x'",
        "Integrity checksum changed for:'/etc/passwd'",
        "Old md5sum was: aaaaaaaa",
        "New md5sum is : bbbbbbbb",
        "Old sha1sum was: cccccccc",
        "Size changed from 10 to 20",
        "Ownership was root",
        "Group ownership was wheel",
        "Permissions changed from 0644 to 0600",
        "arbitrary log message",
        "",
        "\ttabbed line",
        "line with 'quotes' and : colons",
    ];

    let mut out: Vec<u8> = Vec::new();
    let n_alerts = 1 + rng.below(4);
    for _ in 0..n_alerts {
        // Bias towards well-formed headers/dates so that a decent share of the
        // corpus actually reaches the field-parsing code, while still covering
        // the malformed variants.
        let hdr = if rng.below(4) == 0 {
            rng.pick(HEADERS)
        } else {
            rng.pick(&HEADERS[..4])
        };
        let date = if rng.below(4) == 0 {
            rng.pick(DATES)
        } else {
            rng.pick(&DATES[..2])
        };
        out.extend_from_slice(hdr.as_bytes());
        out.push(b'\n');
        out.extend_from_slice(date.as_bytes());
        out.push(b'\n');
        let n_body = rng.below(8);
        for _ in 0..n_body {
            out.extend_from_slice(rng.pick(BODIES).as_bytes());
            out.push(b'\n');
        }
        // Occasionally throw in an over-long line to hit fgets truncation.
        if rng.below(6) == 0 {
            out.extend_from_slice(b"Src IP: ");
            out.extend_from_slice(&vec![b'7'; 900 + rng.below(400)]);
            out.push(b'\n');
        }
    }
    out
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct Step {
    alert: Option<AlertSnap>,
    stream: StreamSnap,
}

fn capture_stderr<F: FnOnce() -> R, R>(f: F) -> (R, Vec<u8>) {
    unsafe {
        let path = std::env::temp_dir().join(format!(
            "c2rust-stderr5-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let mut tmp = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .expect("open capture file");
        use std::os::fd::AsRawFd;
        let cap_fd = tmp.as_raw_fd();

        libc::fflush(std::ptr::null_mut());
        let saved = libc::dup(2);
        assert!(libc::dup2(cap_fd, 2) >= 0);
        let r = f();
        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 2);
        libc::close(saved);

        tmp.seek(SeekFrom::Start(0)).expect("seek");
        let mut out = Vec::new();
        tmp.read_to_end(&mut out).expect("read");
        let _ = std::fs::remove_file(&path);
        (r, out)
    }
}

unsafe fn drain(imp: &Impl, path: &PathBuf, flag: c_int) -> Vec<Step> {
    let mut steps = Vec::new();
    let fp = fopen(path, b"r");
    for _ in 0..24 {
        *libc::__errno_location() = 0;
        let al = (imp.GetAlertData)(flag, fp);
        let snap = snap_alert(al);
        let done = snap.is_none();
        steps.push(Step {
            alert: snap,
            stream: snap_stream(fp),
        });
        if !al.is_null() {
            (imp.FreeAlertData)(al);
        }
        if done {
            break;
        }
    }
    libc::fclose(fp);
    steps
}

#[test]
fn fuzz_get_alert_data_differential() {
    let p = pair();
    let dir = TempDir::new("fuzz");
    let _g = lock();

    for seed in 1u64..=400 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let content = random_log(&mut rng);
        let path = dir.file("alerts.log", &content);
        let flag = *rng.pick(&[0, CRALERT_MAIL_SET, CRALERT_READ_ALL, CRALERT_FP_SET, 0x1f]);

        let (sc, ec) = capture_stderr(|| unsafe { drain(&p.c, &path, flag) });
        let (sr, er) = capture_stderr(|| unsafe { drain(&p.rs, &path, flag) });

        if sc != sr || ec != er {
            panic!(
                "seed {seed} (flag {flag:#x}) mismatch\n--- input ---\n{}\n--- C ---\n{sc:#?}\nstderr: {:?}\n--- Rust ---\n{sr:#?}\nstderr: {:?}",
                String::from_utf8_lossy(&content),
                String::from_utf8_lossy(&ec),
                String::from_utf8_lossy(&er)
            );
        }
    }
}

#[test]
fn fuzz_random_bytes_differential() {
    // Completely unstructured input, biased towards bytes that matter to the
    // parser, to shake out any remaining divergence.
    let p = pair();
    let dir = TempDir::new("fuzzbytes");
    let _g = lock();

    const ALPHABET: &[u8] = b"** Alert:-'\n \tRuleSrcIPDstPortUser0123456789./\x00\xff";

    for seed in 1u64..=300 {
        let mut rng = Rng(seed.wrapping_mul(0xD1B5_4A32_D192_ED03) | 1);
        let len = 1 + rng.below(600);
        let mut content: Vec<u8> = (0..len).map(|_| *rng.pick(ALPHABET)).collect();
        // Guarantee a trailing newline so no line can end mid-buffer without a
        // NUL of its own (see the note on `random_log`).
        content.push(b'\n');
        // Never leave a bare `** Alert` as the very last line.
        let path = dir.file("alerts.log", &content);
        let flag = *rng.pick(&[0, CRALERT_MAIL_SET, 0x1f]);

        let (sc, ec) = capture_stderr(|| unsafe { drain(&p.c, &path, flag) });
        let (sr, er) = capture_stderr(|| unsafe { drain(&p.rs, &path, flag) });

        if sc != sr || ec != er {
            panic!(
                "seed {seed} (flag {flag:#x}) mismatch on random bytes\ninput: {:?}\n--- C ---\n{sc:#?}\nstderr {:?}\n--- Rust ---\n{sr:#?}\nstderr {:?}",
                content,
                String::from_utf8_lossy(&ec),
                String::from_utf8_lossy(&er)
            );
        }
    }
}

#[test]
fn fuzz_driver_differential() {
    // driver() itself, with READ_ALL so it never sleeps.
    let p = pair();
    let dir = TempDir::new("fuzzdriver");
    let _g = lock();
    let old = std::env::current_dir().unwrap();
    std::env::set_current_dir(&dir.0).unwrap();

    let mut failure = None;
    for seed in 1u64..=120 {
        let mut rng = Rng(seed.wrapping_mul(0xA076_1D64_78BD_642F) | 1);
        let content = random_log(&mut rng);
        std::fs::write(dir.0.join("alerts.log"), &content).unwrap();

        let day = rng.below(40) as c_int - 5;
        let mon = rng.below(12) as c_int;
        let year = rng.below(300) as c_int;
        let flags = CRALERT_READ_ALL | *rng.pick(&[0, CRALERT_MAIL_SET, CRALERT_EXEC_SET]);

        let (ac, ec) = capture_stderr(|| unsafe {
            *libc::__errno_location() = 0;
            let a = (p.c.driver)(day, mon, year, 0, flags);
            let s = snap_alert(a);
            if !a.is_null() {
                (p.c.FreeAlertData)(a);
            }
            s
        });
        let (ar, er) = capture_stderr(|| unsafe {
            *libc::__errno_location() = 0;
            let a = (p.rs.driver)(day, mon, year, 0, flags);
            let s = snap_alert(a);
            if !a.is_null() {
                (p.rs.FreeAlertData)(a);
            }
            s
        });

        if ac != ar || ec != er {
            failure = Some(format!(
                "seed {seed} flags {flags:#x} d={day} m={mon} y={year}\ninput:\n{}\nC: {ac:#?} stderr {:?}\nRust: {ar:#?} stderr {:?}",
                String::from_utf8_lossy(&content),
                String::from_utf8_lossy(&ec),
                String::from_utf8_lossy(&er)
            ));
            break;
        }
    }
    std::env::set_current_dir(old).unwrap();
    if let Some(f) = failure {
        panic!("{f}");
    }
}

/// Sanity check that the fuzz corpus really does produce parsed alerts (so a
/// green run is not just comparing NULL against NULL).
#[test]
fn fuzz_corpus_produces_alerts() {
    let p = pair();
    let dir = TempDir::new("fuzzcov");
    let _g = lock();
    let mut parsed = 0usize;
    let mut with_rule = 0usize;
    let mut with_filename = 0usize;
    let mut rewinds = 0usize;

    for seed in 1u64..=400 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let content = random_log(&mut rng);
        let path = dir.file("alerts.log", &content);
        let steps = capture_stderr(|| unsafe { drain(&p.c, &path, 0) }).0;
        for s in &steps {
            if let Some(a) = &s.alert {
                parsed += 1;
                if a.rule != 0 {
                    with_rule += 1;
                }
                if a.filename.is_some() {
                    with_filename += 1;
                }
                // An alert returned while the stream is not at EOF can only
                // have been terminated by the `fseek(-strlen(str))` rewind.
                if !s.stream.eof {
                    rewinds += 1;
                }
            }
        }
    }
    assert!(parsed > 100, "corpus parsed only {parsed} alerts");
    assert!(with_rule > 20, "only {with_rule} alerts carried a rule id");
    assert!(
        with_filename > 5,
        "only {with_filename} alerts carried a syscheck filename"
    );
    assert!(rewinds > 5, "the fseek rewind path was hit only {rewinds} times");
}
