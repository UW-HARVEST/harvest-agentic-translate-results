//! Phase B — valid-path differential tests for the non-`static` functions of
//! `c_src/src/main.c` (`CONFIGS.md` rows C34-C38).

mod common;

use common::*;
use std::ffi::c_char;
use std::io::Write;

// ---------------------------------------------------------------------------
// C34: print_menu
// ---------------------------------------------------------------------------

#[test]
fn c34_print_menu() {
    let _g = lock();
    let p = libs();
    for n in [1usize, 2, 5] {
        let c = p.c.captured(|| {
            for _ in 0..n {
                (p.c.print_menu)()
            }
        });
        let r = p.rust.captured(|| {
            for _ in 0..n {
                (p.rust.print_menu)()
            }
        });
        assert_eq!(show(&c), show(&r), "print_menu x{}", n);
        assert!(!c.is_empty());
    }
}

// ---------------------------------------------------------------------------
// C35: print_analysis_result
// ---------------------------------------------------------------------------

fn diff_print_result(r: CResult) {
    let p = libs();
    let c = p.c.captured(|| (p.c.print_analysis_result)(r));
    let rr = p.rust.captured(|| (p.rust.print_analysis_result)(r));
    assert_eq!(show(&c), show(&rr), "print_analysis_result({:?})", r);
}

#[test]
fn c35_print_analysis_result_values() {
    let _g = lock();
    diff_print_result(CResult::default());
    diff_print_result(CResult {
        word_count: 1,
        number_count: 2,
        keyword_count: 3,
        operator_count: 4,
        comment_count: 5,
        string_count: 6,
        line_count: 7,
        char_count: 8,
    });
    diff_print_result(CResult {
        word_count: usize::MAX,
        number_count: usize::MAX - 1,
        keyword_count: usize::MAX / 2,
        operator_count: 0,
        comment_count: 1,
        string_count: usize::MAX,
        line_count: 0,
        char_count: usize::MAX,
    });

    let mut rng = Rng::new(0xC35);
    for _ in 0..200 {
        let mut f = || match rng.below(4) {
            0 => 0usize,
            1 => rng.below(1000),
            2 => rng.next_u64() as usize,
            _ => usize::MAX - rng.below(4),
        };
        let r = CResult {
            word_count: f(),
            number_count: f(),
            keyword_count: f(),
            operator_count: f(),
            comment_count: f(),
            string_count: f(),
            line_count: f(),
            char_count: f(),
        };
        diff_print_result(r);
    }
}

// ---------------------------------------------------------------------------
// C36: analyze_text -> print_analysis_result pipeline
// ---------------------------------------------------------------------------

#[test]
fn c36_analyze_then_print() {
    let _g = lock();
    let p = libs();
    (p.c.analyzer_init)((p.c.get_tokenizer_ops)());
    (p.rust.analyzer_init)((p.rust.get_tokenizer_ops)());

    let mut rng = Rng::new(0xC36);
    for i in 0..60 {
        let text = if i % 4 == 0 {
            random_soup(&mut rng, 300)
        } else {
            random_source(&mut rng, 40)
        };
        let s = cstring(&text);
        let c = p.c.captured(|| {
            let r = (p.c.analyze_text)(s.as_ptr() as *const c_char);
            (p.c.print_analysis_result)(r);
        });
        let r = p.rust.captured(|| {
            let r = (p.rust.analyze_text)(s.as_ptr() as *const c_char);
            (p.rust.print_analysis_result)(r);
        });
        assert_eq!(show(&c), show(&r), "pipeline differs for {}", show(&text));
        assert_eq!(p.c.stats(), p.rust.stats());
    }
}

// ---------------------------------------------------------------------------
// C37: read_file
// ---------------------------------------------------------------------------

fn diff_read_file(path: &[u8]) -> Option<Vec<u8>> {
    let p = libs();
    let s = cstring(path);
    let pc = (p.c.read_file)(s.as_ptr() as *const c_char);
    let pr = (p.rust.read_file)(s.as_ptr() as *const c_char);

    let a = if pc.is_null() {
        None
    } else {
        Some(read_cstr(pc))
    };
    let b = if pr.is_null() {
        None
    } else {
        Some(read_cstr(pr))
    };
    if !pc.is_null() {
        c_free(pc);
    }
    if !pr.is_null() {
        c_free(pr);
    }
    match (&a, &b) {
        (None, None) => {}
        (Some(x), Some(y)) => assert_eq!(
            show(x),
            show(y),
            "read_file contents differ for {}",
            show(path)
        ),
        _ => panic!(
            "read_file NULL-ness differs for {}: C={:?} Rust={:?}",
            show(path),
            a.as_ref().map(|v| v.len()),
            b.as_ref().map(|v| v.len())
        ),
    }
    a
}

fn read_cstr(p: *mut c_char) -> Vec<u8> {
    let mut v = Vec::new();
    let mut i = 0isize;
    unsafe {
        while *p.offset(i) != 0 {
            v.push(*p.offset(i) as u8);
            i += 1;
        }
    }
    v
}

fn tmpfile(name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("ta_rf_{}_{}", std::process::id(), name));
    let mut f = std::fs::File::create(&path).expect("create temp file");
    f.write_all(bytes).expect("write temp file");
    path
}

#[test]
fn c37_read_file_regular_files() {
    let _g = lock();
    let mut rng = Rng::new(0xC37);

    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("empty", vec![]),
        ("one", b"x".to_vec()),
        ("nl", b"\n".to_vec()),
        ("text", b"int main(void) { return 0; }\n".to_vec()),
        ("no_trailing_nl", b"abc".to_vec()),
        ("crlf", b"a\r\nb\r\n".to_vec()),
        ("nul_inside", b"abc\0def".to_vec()),
        ("nul_first", b"\0abc".to_vec()),
        ("binary", (0u8..=255).collect()),
        ("n8191", vec![b'a'; 8191]),
        ("n8192", vec![b'b'; 8192]),
    ];
    for (name, bytes) in &cases {
        let path = tmpfile(name, bytes);
        let got = diff_read_file(path.as_os_str().as_encoded_bytes());
        println!("{:>16} -> {:?} bytes", name, got.as_ref().map(|v| v.len()));
        let _ = std::fs::remove_file(&path);
    }

    // randomized contents
    for i in 0..40 {
        let bytes = random_soup(&mut rng, 3000);
        let path = tmpfile(&format!("rand{}", i), &bytes);
        diff_read_file(path.as_os_str().as_encoded_bytes());
        let _ = std::fs::remove_file(&path);
    }

    // special files
    diff_read_file(b"/dev/null");
    diff_read_file(b"/proc/version");
    diff_read_file(std::env::temp_dir().as_os_str().as_encoded_bytes()); // a directory
}

// ---------------------------------------------------------------------------
// C38: read_file -> analyze_text pipeline
// ---------------------------------------------------------------------------

#[test]
fn c38_read_file_then_analyze() {
    let _g = lock();
    let p = libs();
    (p.c.analyzer_init)((p.c.get_tokenizer_ops)());
    (p.rust.analyzer_init)((p.rust.get_tokenizer_ops)());

    let mut rng = Rng::new(0xC38);
    for i in 0..30 {
        let mut bytes = Vec::new();
        while bytes.len() < 1500 {
            bytes.extend_from_slice(&random_source(&mut rng, 30));
        }
        let path = tmpfile(&format!("pipe{}", i), &bytes);
        let name = cstring(path.as_os_str().as_encoded_bytes());

        let c = p.c.captured(|| {
            let content = (p.c.read_file)(name.as_ptr() as *const c_char);
            assert!(!content.is_null());
            let r = (p.c.analyze_text)(content);
            (p.c.print_analysis_result)(r);
            c_free(content);
        });
        let r = p.rust.captured(|| {
            let content = (p.rust.read_file)(name.as_ptr() as *const c_char);
            assert!(!content.is_null());
            let r = (p.rust.analyze_text)(content);
            (p.rust.print_analysis_result)(r);
            c_free(content);
        });
        assert_eq!(show(&c), show(&r), "read_file+analyze pipeline #{}", i);
        assert_eq!(p.c.stats(), p.rust.stats());
        let _ = std::fs::remove_file(&path);
    }
}
