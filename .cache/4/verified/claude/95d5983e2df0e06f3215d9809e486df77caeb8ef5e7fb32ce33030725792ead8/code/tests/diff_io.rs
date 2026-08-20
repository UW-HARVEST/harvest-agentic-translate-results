//! Phase B — valid-path differential tests for the stdio entry points
//! `read_buffer` and `write_buffer` (CONFIGS.md rows 72–79).
//!
//! fd 0 is pointed at a scratch file holding the scenario's token stream. The C
//! side additionally `freopen`s `stdin` so glibc starts with a fresh `FILE`
//! buffer at offset 0; the Rust side resets its equivalent reader state.

mod common;

use common::*;

/// One `read_buffer` result: return code + the resulting buffer.
type ReadResult = Vec<(i64, usize, u32, Vec<u8>)>;

fn read_n(api: &Api, calls: usize) -> ReadResult {
    let mut out = Vec::with_capacity(calls);
    for _ in 0..calls {
        // Pre-fill so that "the callee only writes data[0..length]" is visible.
        let mut b = BufferT::patterned(0x5C);
        let rc = unsafe { (api.read_buffer)(&mut b) } as i64;
        out.push((rc, b.length, b.checksum, b.data.to_vec()));
    }
    out
}

#[track_caller]
fn diff_read_buffer(what: &str, input: &[u8], calls: usize) {
    let (c, r) = both();
    // Must be fetched *outside* `observe` (it takes the same lock).
    let path = observe_stdin_path();

    let co = observe(Some(input), || {
        c_freopen_stdin(&path);
        read_n(c, calls)
    });
    let ro = observe(Some(input), || {
        unsafe { (r.reset_stdin.expect("rust reset hook"))() };
        read_n(r, calls)
    });
    same(
        &format!("{} input={:?}", what, String::from_utf8_lossy(input)),
        &co,
        &ro,
    );
}

#[track_caller]
fn diff_write_buffer(what: &str, b: BufferT) {
    let (c, r) = both();
    let cb = b;
    let rb = b;
    let co = observe(None, || unsafe { (c.write_buffer)(&cb) });
    let ro = observe(None, || unsafe { (r.write_buffer)(&rb) });
    same(what, &co, &ro);
    same_buf(what, &cb, &rb); // write_buffer must not modify its argument
}

fn tokens_to_input(toks: &[i64], rng: &mut Rng) -> Vec<u8> {
    let seps: [&str; 8] = [" ", "\n", "\t", "  ", " \n ", "\r\n", "\x0b", "\x0c"];
    let mut s = String::new();
    for (i, t) in toks.iter().enumerate() {
        if i > 0 {
            s.push_str(rng.pick(&seps));
        }
        if *t >= 0 && rng.below(10) == 0 {
            s.push('+');
        } else if *t >= 0 && rng.below(12) == 0 {
            s.push('0'); // leading zero
        }
        s.push_str(&t.to_string());
    }
    s.push('\n');
    s.into_bytes()
}

// ============================================================== row 72 =====

#[test]
fn row72_read_buffer_length_zero() {
    diff_read_buffer("row72/plain", b"0\n", 1);
    diff_read_buffer("row72/spaces", b"   0   \n", 1);
    diff_read_buffer("row72/trailing", b"0 1 2 3\n", 1);
    diff_read_buffer("row72/twice", b"0\n0\n", 2);
}

// ============================================================== row 73 =====

#[test]
fn row73_read_buffer_random_lengths() {
    let mut rng = Rng::new(0x73);
    for _ in 0..250 {
        let len = 1 + rng.below(64);
        let mut toks: Vec<i64> = vec![len as i64];
        for _ in 0..len {
            toks.push(rng.below(256) as i64);
        }
        let input = tokens_to_input(&toks, &mut rng);
        diff_read_buffer("row73", &input, 1);
    }
}

// ============================================================== row 74 =====

#[test]
fn row74_read_buffer_max_length() {
    let mut rng = Rng::new(0x74);
    for _ in 0..10 {
        for len in [255usize, 256] {
            let mut toks: Vec<i64> = vec![len as i64];
            for _ in 0..len {
                toks.push(rng.below(256) as i64);
            }
            let input = tokens_to_input(&toks, &mut rng);
            diff_read_buffer("row74", &input, 1);
        }
    }
}

// ============================================================== row 75 =====

#[test]
fn row75_read_buffer_byte_values_out_of_range() {
    // `buf->data[i] = (uint8_t)byte;` truncates whatever `%d` produced.
    let mut rng = Rng::new(0x75);
    let specials: [i64; 20] = [
        -1,
        -2,
        -128,
        -129,
        -255,
        -256,
        -257,
        255,
        256,
        257,
        511,
        512,
        1000,
        65535,
        65536,
        2147483647,
        -2147483648,
        2147483648,
        4294967296,
        -4294967296,
    ];
    for _ in 0..80 {
        let len = 1 + rng.below(12);
        let mut toks: Vec<i64> = vec![len as i64];
        for _ in 0..len {
            if rng.bool() {
                toks.push(rng.pick(&specials));
            } else {
                toks.push(rng.below(256) as i64);
            }
        }
        let input = tokens_to_input(&toks, &mut rng);
        diff_read_buffer("row75", &input, 1);
    }
    // every special value on its own
    for s in specials {
        let input = format!("1 {}\n", s).into_bytes();
        diff_read_buffer("row75/single", &input, 1);
    }
    // values that overflow `long` in scanf
    for s in [
        "99999999999999999999",
        "-99999999999999999999",
        "184467440737095516160",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775808",
        "-9223372036854775809",
    ] {
        let input = format!("1 {}\n", s).into_bytes();
        diff_read_buffer("row75/overflow", &input, 1);
    }
}

// ============================================================== row 76 =====

#[test]
fn row76_read_buffer_separator_variants() {
    let ws: [&str; 10] = [
        " ", "\t", "\n", "\r", "\x0b", "\x0c", "   ", "\n\n\n", " \t\n\x0b\x0c\r ", "\t \t",
    ];
    for w in ws {
        let input = format!("{w}3{w}1{w}2{w}3{w}").into_bytes();
        diff_read_buffer("row76", &input, 1);
    }
    // leading '+' and leading zeros on every token
    diff_read_buffer("row76/plus", b"+3 +1 +2 +3\n", 1);
    diff_read_buffer("row76/zeros", b"003 001 0002 00000003\n", 1);
    diff_read_buffer("row76/mixed", b"+0003 -0 +0 00 0\n", 1);
    // no trailing newline at all
    diff_read_buffer("row76/noeol", b"2 7 8", 1);
    // token immediately followed by EOF
    diff_read_buffer("row76/eof", b"1 5", 1);
}

// ============================================================== row 77 =====

#[test]
fn row77_read_buffer_repeated_on_one_stream() {
    let mut rng = Rng::new(0x77);
    for _ in 0..60 {
        let n = 1 + rng.below(6);
        let mut toks: Vec<i64> = Vec::new();
        for _ in 0..n {
            let len = rng.below(10);
            toks.push(len as i64);
            for _ in 0..len {
                toks.push(rng.below(256) as i64);
            }
        }
        let input = tokens_to_input(&toks, &mut rng);
        // Ask for one call more than the stream provides so the EOF path after
        // a successful sequence is covered too.
        diff_read_buffer("row77", &input, n + 1);
    }
}

// ============================================================== row 78 =====

#[test]
fn row78_write_buffer_length_zero() {
    let mut b = BufferT::patterned(0x7E);
    b.length = 0;
    diff_write_buffer("row78/patterned", b);
    diff_write_buffer("row78/zeroed", BufferT::zeroed());
}

// ============================================================== row 79 =====

#[test]
fn row79_write_buffer_decimal_widths() {
    // Byte values whose decimal rendering changes width.
    for v in [0u8, 1, 9, 10, 11, 99, 100, 101, 127, 128, 199, 200, 254, 255] {
        for len in [1usize, 2, 3, 17, 255, 256] {
            let mut b = BufferT::zeroed();
            b.data = [v; 256];
            b.length = len;
            b.checksum = checksum(&b.data[..len]);
            diff_write_buffer("row79/uniform", b);
        }
    }
    let mut rng = Rng::new(0x79);
    for _ in 0..150 {
        let len = rng.below(257);
        let b = rng.buffer_len(len);
        diff_write_buffer("row79/rand", b);
    }
    // ascending / descending byte ramps at maximum length (long output line)
    let mut b = BufferT::zeroed();
    for i in 0..256 {
        b.data[i] = i as u8;
    }
    b.length = 256;
    b.checksum = checksum(&b.data);
    diff_write_buffer("row79/ramp", b);
    for i in 0..256 {
        b.data[i] = 255 - i as u8;
    }
    b.checksum = checksum(&b.data);
    diff_write_buffer("row79/ramp-desc", b);
}

// ---------------------------------------------------------------------------
// Extra: `read_buffer` followed by `write_buffer` — the round trip a real
// consumer performs, which is also what `main` does internally.
// ---------------------------------------------------------------------------

#[test]
fn read_then_write_round_trip() {
    let (c, r) = both();
    let path = observe_stdin_path();
    let mut rng = Rng::new(0xB0);
    for _ in 0..120 {
        let len = rng.below(80);
        let mut toks: Vec<i64> = vec![len as i64];
        for _ in 0..len {
            toks.push(rng.range(-400, 700));
        }
        let input = tokens_to_input(&toks, &mut rng);

        let co = observe(Some(&input), || unsafe {
            c_freopen_stdin(&path);
            let mut b = BufferT::patterned(0x3C);
            let rc = (c.read_buffer)(&mut b) as i64;
            (c.write_buffer)(&b);
            (rc, b.length, b.checksum, b.data.to_vec())
        });
        let ro = observe(Some(&input), || unsafe {
            (r.reset_stdin.unwrap())();
            let mut b = BufferT::patterned(0x3C);
            let rc = (r.read_buffer)(&mut b) as i64;
            (r.write_buffer)(&b);
            (rc, b.length, b.checksum, b.data.to_vec())
        });
        same("read_then_write", &co, &ro);
    }
}
