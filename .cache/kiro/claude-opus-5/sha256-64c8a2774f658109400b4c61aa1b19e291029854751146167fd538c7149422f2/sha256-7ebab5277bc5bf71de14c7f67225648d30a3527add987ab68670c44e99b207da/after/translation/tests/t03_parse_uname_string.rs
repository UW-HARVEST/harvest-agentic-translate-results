//! Level 3: `parse_uname_string` — the public API from `lib.h`.
//!
//! The C writes `*(p + strlen(p) - 1) = '\0'` in four places without checking
//! for an empty string, so an empty `p` writes one byte *before* a 1-byte
//! `malloc` block. The Rust reproduces that instruction for instruction, but
//! the resulting heap corruption is not observable in a well-defined way from
//! either side, so `triggers_strip_underflow` filters those inputs out of the
//! differential comparison rather than pretending they can be compared.

mod common;

use common::*;

/// Returns true if the C would apply `*(p + strlen(p) - 1) = 0` to an empty
/// string for this input, mirroring the branch structure of the C exactly.
fn triggers_strip_underflow(input: &[u8]) -> bool {
    fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
        if needle.is_empty() {
            return Some(0);
        }
        hay.windows(needle.len()).position(|w| w == needle)
    }

    if let Some(pos) = find(input, b" [Ver: ") {
        // str_tmp = &input[pos + 7]
        return input.len() <= pos + 7;
    }

    if let Some(pos) = find(input, b" [") {
        let name = &input[pos + 2..];
        if name.is_empty() {
            return true; // strip_last_char(os_name) on ""
        }
        if let Some(p) = find(name, b": ") {
            let version = &name[p + 2..];
            if version.is_empty() {
                return true; // strip_last_char(os_version) on ""
            }
            let stripped = &version[..version.len() - 1];
            if let Some(q) = find(stripped, b" (") {
                let codename = &stripped[q + 2..];
                if codename.is_empty() {
                    return true; // strip_last_char(os_codename) on ""
                }
            }
        }
    }

    false
}

fn uname_inputs() -> Vec<Vec<u8>> {
    let mut v: Vec<&str> = Vec::new();

    // ---- Windows branch: " [Ver: " ----
    v.push("Microsoft Windows 10 Pro [Ver: 10.0.19045.3803]");
    v.push("Microsoft Windows Server 2019 Datacenter [Ver: 10.0.17763.4974]");
    v.push("Microsoft Windows 7 Professional [Ver: 6.1.7601]");
    v.push("Microsoft Windows XP [Ver: 5.1.2600]");
    v.push("Microsoft Windows 11 Enterprise [Ver: 10.0.22621.2861]");
    // Build with many dotted components (exercises the (\.[0-9]+)* group).
    v.push("Win [Ver: 1.2.3.4.5.6.7]");
    // Only a major.
    v.push("Win [Ver: 10]");
    // Major.minor, no build.
    v.push("Win [Ver: 10.0]");
    // Trailing dot after the build.
    v.push("Win [Ver: 10.0.1.]");
    // Non-numeric version: all three regexes fail, os_version still set.
    v.push("Win [Ver: abc]");
    v.push("Win [Ver: v10.0]");
    // Empty os_name (uname begins with the marker).
    v.push(" [Ver: 10.0.19045]");
    // The marker appears after another bracket; " [Ver: " still wins.
    v.push("Linux [foo] bar [Ver: 1.2.3]");
    // Two markers: strstr finds the first.
    v.push("A [Ver: 1.2.3] B [Ver: 4.5.6]");
    // An architecture is present but the Windows branch never sets os_arch.
    v.push("Windows x86_64 [Ver: 10.0.1]");
    v.push("Windows amd64 [Ver: 6.3.9600]");
    // The bracket content has no closing ']' — the last byte is stripped anyway.
    v.push("Win [Ver: 10.0.19045");
    // Leading zeros and a very long numeric run.
    v.push("Win [Ver: 0010.0002.0003]");
    // Version contains " (" and "|", which only matter in the other branch.
    v.push("Win [Ver: 10.0 (build)]");
    v.push("Win|plat [Ver: 10.0.1]");

    // ---- Unix branch: " [" with ": " ----
    v.push("Linux |ubuntu |5.15.0-91-generic |#101-Ubuntu SMP |x86_64 [Ubuntu|ubuntu: 22.04.3 LTS (Jammy Jellyfish)]");
    v.push("Linux |deb |6.1.0-17-amd64 |#1 SMP |x86_64 [Debian GNU/Linux|debian: 12 (bookworm)]");
    v.push("Linux |centos |3.10.0 |#1 SMP |x86_64 [CentOS Linux|centos: 7.9.2009 (Core)]");
    v.push("Linux |rhel |4.18.0 |#1 SMP |x86_64 [Red Hat Enterprise Linux|rhel: 8.9 (Ootpa)]");
    v.push("Linux |alpine |6.1.0 |#1 SMP |x86_64 [Alpine Linux|alpine: 3.19.0]");
    v.push("Darwin |mac.local |23.2.0 |Darwin Kernel Version 23.2.0 |arm64 [macOS|darwin: 14.2.1 (Sonoma)]");
    v.push("SunOS |solaris |5.11 |11.4 |i86pc [SunOS|sunos: 11.4]");
    v.push("AIX |aix71 |1 |7 [AIX|aix: 7.1]");
    v.push("FreeBSD |bsd |13.2-RELEASE |#0 |amd64 [FreeBSD|freebsd: 13.2]");
    // No "|" in the name: os_platform stays NULL.
    v.push("Linux |host |5.10 |#1 |x86_64 [Ubuntu: 20.04 (focal)]");
    // No " (": os_codename stays NULL.
    v.push("Linux |host |5.10 |#1 |x86_64 [Ubuntu|ubuntu: 20.04]");
    // Non-numeric version: major/minor stay NULL.
    v.push("Linux |host |5.10 |#1 |x86_64 [Rolling|arch: rolling]");
    // Version with only a major.
    v.push("Linux |host |5.10 |#1 |x86_64 [Foo|foo: 12]");
    // Several ": " occurrences — strstr takes the first.
    v.push("Linux |host |x86_64 [A: B: C (D)]");
    // Several " (" occurrences — strstr takes the first.
    v.push("Linux |host |x86_64 [A|a: 1.2 (one) (two)]");
    // Several "|" in the name — strstr takes the first.
    v.push("Linux |host |x86_64 [A|b|c: 1.2]");
    // "|" placed so the platform is empty.
    v.push("Linux |host |x86_64 [A|: 1.2]");
    // Name is empty after the ": " split.
    v.push("Linux |host |x86_64 [: 1.2 (x)]");
    // Bracket content without a trailing ']'.
    v.push("Linux |host |x86_64 [Ubuntu|ubuntu: 20.04 (focal)");
    // Multiple " [" — strstr takes the first.
    v.push("Linux [a: 1.0] [b: 2.0]");
    // " (" inside the codename part.
    v.push("Linux |x86_64 [A|a: 1.2 (a (b))]");

    // ---- Unix branch: " [" without ": " ----
    v.push("Linux |host |5.10 |#1 |x86_64 [Ubuntu]");
    v.push("Linux |host |x86_64 [Ubuntu|ubuntu]");
    v.push("Linux |host |x86_64 [x]");
    v.push("Linux |host |aarch64 [some name without colon-space]");
    // "|" with an empty platform.
    v.push("Linux |host |x86_64 [name|]");

    // ---- No bracket at all: only os_arch ----
    v.push("");
    v.push("Linux");
    v.push("Linux host 5.10 x86_64");
    v.push("Linux |host |5.15.0 |#1 SMP |i686");
    v.push("Darwin mac 23.2.0 arm64");
    v.push("no arch and no bracket");
    v.push("[bracket without leading space]");
    v.push("trailing space ");
    v.push(" leading space");
    v.push("[Ver: 1.2.3]");   // no leading space, so not the Windows marker
    v.push("x [Ver:1.2.3]");  // missing the space after the colon
    v.push("x [ Ver: 1.2.3]");
    v.push("|only a pipe|x86_64");
    v.push(": only a colon-space x86_64");

    // ---- Edge shapes around the markers ----
    v.push(" [x]");
    v.push(" [x: y]");
    v.push(" [x: y (z)]");
    v.push(" [|: 1]");
    v.push("a [b: c (d)] e");
    v.push("a  [b: c]"); // double space before the bracket
    v.push("\t[tab before bracket]");
    v.push("a [b: 1.2.3.4.5 (c d e)] x86_64");

    let mut out: Vec<Vec<u8>> = v.into_iter().map(|s| s.as_bytes().to_vec()).collect();

    // Non-UTF-8 bytes must be carried through untouched.
    out.push(b"Linux \xff\xfe |x86_64 [Ubuntu|ubuntu: 22.04 (\xc3\xa9t\xc3\xa9)]".to_vec());
    out.push(b"Win\xff [Ver: 10.0.1\xff]".to_vec());
    out.push(vec![0x80, 0x81, b' ', b'[', b'a', b':', b' ', b'1', b']']);

    out
}

/// Deterministic pseudo-random strings built from marker-rich fragments, to
/// shake out branch interactions the hand-written list may have missed.
fn fuzz_inputs(count: usize) -> Vec<Vec<u8>> {
    const FRAGMENTS: [&str; 24] = [
        " [", "]", " [Ver: ", ": ", " (", ")", "|", ".", "0", "12", "345", " ", "x86_64", "arm64",
        "AIX", "Linux", "Ubuntu", "a", "Ver: ", "[", "(", "-", "\t", "9.9.9",
    ];

    let mut state: u64 = 0x243f_6a88_85a3_08d3;
    let mut next = |n: usize| -> usize {
        // xorshift64*
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        (state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 33) as usize % n
    };

    let mut out = Vec::with_capacity(count);
    while out.len() < count {
        let parts = 1 + next(9);
        let mut s = String::new();
        for _ in 0..parts {
            s.push_str(FRAGMENTS[next(FRAGMENTS.len())]);
        }
        out.push(s.into_bytes());
    }
    out
}

fn compare(c: &Impl, rust: &Impl, input: &[u8]) {
    let (buf_c, snap_c) = c.parse_uname_string(input);
    let (buf_rust, snap_rust) = rust.parse_uname_string(input);

    assert_eq!(
        buf_c,
        buf_rust,
        "in-place mutation of uname differs for {:?}\nC    = {:?}\nRust = {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&buf_c),
        String::from_utf8_lossy(&buf_rust),
    );

    for i in 0..9 {
        assert_eq!(
            snap_c[i],
            snap_rust[i],
            "field {} differs for input {:?}: C = {}, Rust = {}",
            FIELD_NAMES[i],
            String::from_utf8_lossy(input),
            show(&snap_c[i]),
            show(&snap_rust[i]),
        );
    }
}

#[test]
fn parse_uname_string_matches_c() {
    let (c, rust) = load_both();
    let mut checked = 0usize;

    for input in uname_inputs() {
        if triggers_strip_underflow(&input) {
            continue;
        }
        compare(&c, &rust, &input);
        checked += 1;
    }

    assert!(checked > 60, "expected a broad corpus, only ran {checked}");
}

#[test]
fn parse_uname_string_matches_c_on_fuzz_corpus() {
    let (c, rust) = load_both();
    let mut checked = 0usize;

    for input in fuzz_inputs(4000) {
        if triggers_strip_underflow(&input) {
            continue;
        }
        compare(&c, &rust, &input);
        checked += 1;
    }

    assert!(checked > 1000, "fuzz corpus too small: {checked}");
}

#[test]
fn parse_uname_string_null_osd_matches_c() {
    // With osd == NULL the C returns immediately and must not touch uname.
    let (c, rust) = load_both();

    for input in uname_inputs() {
        let buf_c = c.parse_uname_string_null_osd(&input);
        let buf_rust = rust.parse_uname_string_null_osd(&input);
        assert_eq!(
            buf_c,
            buf_rust,
            "NULL osd differs for {:?}",
            String::from_utf8_lossy(&input)
        );

        let mut expected: Vec<u8> = input.clone();
        expected.push(0);
        assert_eq!(buf_c, expected, "C modified uname despite a NULL osd");
    }
}

#[test]
fn parse_uname_string_leaves_prefilled_fields_alone_identically() {
    // The C only ever assigns fields it parses; unset ones keep their previous
    // value. Confirm both implementations agree on which fields get written by
    // checking that the NULL/non-NULL pattern is the same.
    let (c, rust) = load_both();

    for input in uname_inputs() {
        if triggers_strip_underflow(&input) {
            continue;
        }
        let (_, snap_c) = c.parse_uname_string(&input);
        let (_, snap_rust) = rust.parse_uname_string(&input);
        let pat_c: Vec<bool> = snap_c.iter().map(|f| f.is_some()).collect();
        let pat_rust: Vec<bool> = snap_rust.iter().map(|f| f.is_some()).collect();
        assert_eq!(
            pat_c,
            pat_rust,
            "different set of populated fields for {:?}",
            String::from_utf8_lossy(&input)
        );
    }
}
