// Phase B — valid-path differential tests, rows C1..C12 of CONFIGS.md.
//
// These drive the LOW-LEVEL exports directly (not through `findrep`), starting
// from the bottom of the call hierarchy and working upward. Every row uses
// many randomized inputs from a fixed seed.

mod common;
use common::*;

const SEED: u64 = 0x5EED_1234_ABCD_0001;

// ===========================================================================
// C1 / C2 — validate_and_normalize
// ===========================================================================

#[test]
fn c1_validate_and_normalize_buckets_and_random() {
    let p = LibPair::fresh("c1");
    let (c, r) = p.apis();

    // The 9 value buckets from CONFIGS.md, one representative each plus the
    // bucket edges.
    let buckets: &[i32] = &[
        0, // value == 0
        1, 7, 63, // 0 < value < 64
        64, // value == 64 (== 0o100)
        65, 100, 300, 510, // 64 < value < 511
        511,  // value == 511 (== 0o777)
        512, 1000, 1 << 20, // value > 511
        -1, -64, -511, -1000, // value < 0
        i32::MIN,
        i32::MAX,
    ];
    for &v in buckets {
        let cv = unsafe { (c.validate_and_normalize)(v) };
        let rv = unsafe { (r.validate_and_normalize)(v) };
        assert_eq!(cv, rv, "validate_and_normalize({v}): C={cv} Rust={rv}");
    }

    let mut rng = Rng::new(SEED);
    for _ in 0..4096 {
        let v = rng.next_i32();
        let cv = unsafe { (c.validate_and_normalize)(v) };
        let rv = unsafe { (r.validate_and_normalize)(v) };
        assert_eq!(cv, rv, "validate_and_normalize({v}): C={cv} Rust={rv}");
    }
    for _ in 0..4096 {
        let v = rng.interesting_i32();
        let cv = unsafe { (c.validate_and_normalize)(v) };
        let rv = unsafe { (r.validate_and_normalize)(v) };
        assert_eq!(cv, rv, "validate_and_normalize({v}): C={cv} Rust={rv}");
    }
}

#[test]
fn c2_validate_and_normalize_boundary_neighbourhoods() {
    let p = LibPair::fresh("c2");
    let (c, r) = p.apis();
    let mut vals: Vec<i32> = Vec::new();
    for centre in [0i32, 1, -1, 0o100, 0o777, 0o150, 0o10, 100, 512] {
        for d in -4i32..=4 {
            vals.push(centre.wrapping_add(d));
        }
    }
    for d in 0i32..=4 {
        vals.push(i32::MIN.wrapping_add(d));
        vals.push(i32::MAX.wrapping_sub(d));
    }
    for &v in &vals {
        let cv = unsafe { (c.validate_and_normalize)(v) };
        let rv = unsafe { (r.validate_and_normalize)(v) };
        assert_eq!(cv, rv, "validate_and_normalize({v}): C={cv} Rust={rv}");
    }
}

// ===========================================================================
// C3 — process_octal_string
// ===========================================================================

fn cmp_octal(c: &Api<'_>, r: &Api<'_>, v: i32) {
    // Two independently pre-poisoned buffers: any difference in how many bytes
    // the callee writes (or where it puts the terminator) shows up as a
    // trailing-byte mismatch, not just a string mismatch.
    for fill in [0xAAu8, 0x00, 0xFF] {
        let mut cb = scratch(fill);
        let mut rb = scratch(fill);
        unsafe { (c.process_octal_string)(cb.as_mut_ptr(), v) };
        unsafe { (r.process_octal_string)(rb.as_mut_ptr(), v) };
        assert_eq!(
            as_u8(&cb),
            as_u8(&rb),
            "process_octal_string(buf[fill=0x{fill:02x}], {v}):\n  C   ={}\n  Rust={}",
            show(&cb),
            show(&rb)
        );
    }
}

#[test]
fn c3_process_octal_string_shapes_and_random() {
    let p = LibPair::fresh("c3");
    let (c, r) = p.apis();

    // the 7 documented value shapes
    for v in [
        0,
        1,
        7,
        8,
        0o123,
        0o777,
        0o7777_7777_7,
        i32::MAX,
        -1,
        -2,
        -0o123,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX - 1,
        1 << 30,
        (1i64 << 31) as i32,
    ] {
        cmp_octal(&c, &r, v);
    }

    let mut rng = Rng::new(SEED ^ 0x33);
    for _ in 0..2048 {
        cmp_octal(&c, &r, rng.next_i32());
    }
    for _ in 0..512 {
        cmp_octal(&c, &r, rng.interesting_i32());
    }
}

// ===========================================================================
// C4 / C5 / C6 / C7 — find_and_replace_char
// ===========================================================================

fn cmp_replace(c: &Api<'_>, r: &Api<'_>, s: &[u8], needle: i32) {
    for fill in [0xAAu8, 0x5B] {
        let mut cb = scratch(fill);
        let mut rb = scratch(fill);
        set_cstr(&mut cb, s);
        set_cstr(&mut rb, s);
        unsafe { (c.find_and_replace_char)(cb.as_mut_ptr(), needle) };
        unsafe { (r.find_and_replace_char)(rb.as_mut_ptr(), needle) };
        assert_eq!(
            as_u8(&cb),
            as_u8(&rb),
            "find_and_replace_char({:?}, {needle}):\n  C   ={}\n  Rust={}",
            String::from_utf8_lossy(s),
            show(&cb),
            show(&rb)
        );
    }
}

#[test]
fn c4_replace_needle_at_first_middle_last() {
    let p = LibPair::fresh("c4");
    let (c, r) = p.apis();
    let mut rng = Rng::new(SEED ^ 0x44);

    for _ in 0..600 {
        let len = 1 + rng.below(60) as usize;
        // random printable ASCII avoiding the needle we will inject
        let mut s: Vec<u8> = (0..len)
            .map(|_| b'a' + (rng.below(26) as u8))
            .collect();
        let needle = b'Z';
        for pos in [0usize, len / 2, len - 1] {
            let mut t = s.clone();
            t[pos] = needle;
            cmp_replace(&c, &r, &t, needle as i32);
        }
        // needle at every position of a short string
        if len <= 8 {
            for pos in 0..len {
                let mut t = s.clone();
                t[pos] = needle;
                cmp_replace(&c, &r, &t, needle as i32);
            }
        }
        s[0] = b'q';
        cmp_replace(&c, &r, &s, b'q' as i32);
    }

    // the literal string findrep itself searches, and the message it builds
    cmp_replace(
        &c,
        &r,
        b"Function pointer example with static vars",
        b'p' as i32,
    );
    cmp_replace(&c, &r, b"Octal: 0123, Decimal: 83", b'O' as i32);
}

#[test]
fn c5_replace_absent_repeated_empty_and_len1() {
    let p = LibPair::fresh("c5");
    let (c, r) = p.apis();

    // empty string
    cmp_replace(&c, &r, b"", b'a' as i32);
    cmp_replace(&c, &r, b"", 0);
    cmp_replace(&c, &r, b"", 255);
    // length 1, hit and miss
    cmp_replace(&c, &r, b"a", b'a' as i32);
    cmp_replace(&c, &r, b"a", b'b' as i32);
    cmp_replace(&c, &r, b"X", b'X' as i32);
    // absent
    cmp_replace(&c, &r, b"hello world", b'Q' as i32);
    cmp_replace(&c, &r, b"hello world", b'\n' as i32);
    // repeated -> only the FIRST occurrence is replaced
    cmp_replace(&c, &r, b"aaaaaaa", b'a' as i32);
    cmp_replace(&c, &r, b"abcabcabc", b'b' as i32);
    cmp_replace(&c, &r, b"zzzzzzzzzzzzzzzzzzzzzzzz", b'z' as i32);
    // already contains the replacement char 'X'
    cmp_replace(&c, &r, b"XXXaXXX", b'a' as i32);
    cmp_replace(&c, &r, b"XXXXXXX", b'X' as i32);
    // long string, needle only at the very end
    let mut long = vec![b'.'; 200];
    long[199] = b'!';
    cmp_replace(&c, &r, &long, b'!' as i32);
    cmp_replace(&c, &r, &long, b'?' as i32);
}

#[test]
fn c6_replace_high_bytes_signed_char_path() {
    let p = LibPair::fresh("c6");
    let (c, r) = p.apis();
    let mut rng = Rng::new(SEED ^ 0x66);

    // Strings made only of bytes >= 0x80 (negative as signed char) with the
    // needle also in that range: this is where char-signedness in the memchr
    // comparison would show up.
    for _ in 0..600 {
        let len = 1 + rng.below(40) as usize;
        let s: Vec<u8> = (0..len)
            .map(|_| 0x80u8 + (rng.below(0x80) as u8))
            .collect();
        let needle_byte = s[rng.below(len as u64) as usize];
        cmp_replace(&c, &r, &s, needle_byte as i32);
        // as a sign-extended negative int too
        cmp_replace(&c, &r, &s, (needle_byte as i8) as i32);
        // an absent high byte
        cmp_replace(&c, &r, &s, 0x80i32.wrapping_sub(1));
    }

    // every single high byte, present
    for b in 0x80u8..=0xFF {
        let s = [b'a', b, b'z'];
        cmp_replace(&c, &r, &s, b as i32);
        cmp_replace(&c, &r, &s, (b as i8) as i32);
    }
    // mixed low+high bytes
    for _ in 0..300 {
        let len = 1 + rng.below(50) as usize;
        let s: Vec<u8> = (0..len)
            .map(|_| {
                let v = rng.below(255) as u8 + 1; // 1..=255, never NUL
                v
            })
            .collect();
        let needle = rng.next_i32();
        cmp_replace(&c, &r, &s, needle);
    }
}

#[test]
fn c7_replace_randomized_full_domain() {
    let p = LibPair::fresh("c7");
    let (c, r) = p.apis();
    let mut rng = Rng::new(SEED ^ 0x77);
    for _ in 0..2048 {
        let len = rng.below(80) as usize; // includes 0
        let s: Vec<u8> = (0..len).map(|_| (rng.below(255) as u8) + 1).collect();
        let needle = match rng.below(3) {
            0 => rng.next_i32(),
            1 => rng.below(512) as i32 - 256,
            _ => {
                if len == 0 {
                    0
                } else {
                    s[rng.below(len as u64) as usize] as i32
                }
            }
        };
        cmp_replace(&c, &r, &s, needle);
    }
}

// ===========================================================================
// C8..C11 — the four state operations, each as a fresh randomized sequence
// ===========================================================================

#[test]
fn c8_add_to_accumulator_sequence() {
    let p = LibPair::fresh("c8");
    let (c, r) = p.apis();
    let mut rng = Rng::new(SEED ^ 0x88);
    for i in 0..4096 {
        let (a, b) = if i % 3 == 0 {
            (rng.interesting_i32(), rng.interesting_i32())
        } else {
            (rng.next_i32(), rng.next_i32())
        };
        let cv = unsafe { (c.add_to_accumulator)(a, b) };
        let rv = unsafe { (r.add_to_accumulator)(a, b) };
        assert_eq!(cv, rv, "step {i}: add_to_accumulator({a},{b}) C={cv} Rust={rv}");
    }
}

#[test]
fn c9_multiply_with_multiplier_sequence() {
    let p = LibPair::fresh("c9");
    let (c, r) = p.apis();
    let mut rng = Rng::new(SEED ^ 0x99);
    for i in 0..4096 {
        let (a, b) = if i % 3 == 0 {
            (rng.interesting_i32(), rng.interesting_i32())
        } else {
            (rng.next_i32(), rng.next_i32())
        };
        let cv = unsafe { (c.multiply_with_multiplier)(a, b) };
        let rv = unsafe { (r.multiply_with_multiplier)(a, b) };
        assert_eq!(
            cv, rv,
            "step {i}: multiply_with_multiplier({a},{b}) C={cv} Rust={rv}"
        );
    }
}

#[test]
fn c10_subtract_from_accumulator_sequence() {
    let p = LibPair::fresh("c10");
    let (c, r) = p.apis();
    let mut rng = Rng::new(SEED ^ 0xAA);
    for i in 0..4096 {
        let (a, b) = if i % 3 == 0 {
            (rng.interesting_i32(), rng.interesting_i32())
        } else {
            (rng.next_i32(), rng.next_i32())
        };
        let cv = unsafe { (c.subtract_from_accumulator)(a, b) };
        let rv = unsafe { (r.subtract_from_accumulator)(a, b) };
        assert_eq!(
            cv, rv,
            "step {i}: subtract_from_accumulator({a},{b}) C={cv} Rust={rv}"
        );
    }
}

#[test]
fn c11_divide_multiplier_sequence() {
    let p = LibPair::fresh("c11");
    let (c, r) = p.apis();
    let mut rng = Rng::new(SEED ^ 0xBB);
    // Track `multiplier` exactly (only multiply/divide touch it) so we can
    // avoid the one input pair that makes the C hardware-trap: INT_MIN / -1
    // (see ERRORS.md row E3).
    let mut mult: i32 = 1;
    for i in 0..4096 {
        let a = rng.next_i32();
        let mut b = match rng.below(4) {
            0 => 0, // the rejection branch: division skipped
            1 => [1i32, -1, 2, -2, 3, 7, i32::MIN, i32::MAX][rng.below(8) as usize],
            2 => (rng.below(64) as i32) - 32,
            _ => rng.next_i32(),
        };
        if mult == i32::MIN && b == -1 {
            b = 1;
        }
        let cv = unsafe { (c.divide_multiplier)(a, b) };
        let rv = unsafe { (r.divide_multiplier)(a, b) };
        assert_eq!(
            cv, rv,
            "step {i}: divide_multiplier({a},{b}) [mult={mult}] C={cv} Rust={rv}"
        );
        mult = cv;

        // occasionally re-seed the multiplier so `mult` keeps moving instead of
        // collapsing to 0 forever
        if i % 37 == 0 {
            let (x, y) = (rng.interesting_i32(), rng.interesting_i32());
            let cm = unsafe { (c.multiply_with_multiplier)(x, y) };
            let rm = unsafe { (r.multiply_with_multiplier)(x, y) };
            assert_eq!(cm, rm, "step {i}: reseed multiply({x},{y}) C={cm} Rust={rm}");
            mult = cm;
        }
    }
}

// ===========================================================================
// C12 — interleaved four-operation state machine
// ===========================================================================

#[test]
fn c12_interleaved_four_operations() {
    let p = LibPair::fresh("c12");
    let (c, r) = p.apis();
    let mut rng = Rng::new(SEED ^ 0xCC);
    for i in 0..8192 {
        let (a, b) = if i % 2 == 0 {
            (rng.interesting_i32(), rng.interesting_i32())
        } else {
            (rng.next_i32(), rng.next_i32())
        };
        let which = rng.below(4);
        let (cv, rv, name) = unsafe {
            match which {
                0 => (
                    (c.add_to_accumulator)(a, b),
                    (r.add_to_accumulator)(a, b),
                    "add",
                ),
                1 => (
                    (c.multiply_with_multiplier)(a, b),
                    (r.multiply_with_multiplier)(a, b),
                    "multiply",
                ),
                2 => (
                    (c.subtract_from_accumulator)(a, b),
                    (r.subtract_from_accumulator)(a, b),
                    "subtract",
                ),
                _ => {
                    // never pass b == -1, so INT_MIN / -1 cannot occur
                    let b = if b == -1 { 1 } else { b };
                    (
                        (c.divide_multiplier)(a, b),
                        (r.divide_multiplier)(a, b),
                        "divide",
                    )
                }
            }
        };
        assert_eq!(cv, rv, "step {i}: {name}({a},{b}) C={cv} Rust={rv}");
    }
}
