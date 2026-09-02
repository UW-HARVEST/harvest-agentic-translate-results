//! Phase C — the size limits and buffer edges: MAX_TOKEN_LENGTH (256),
//! MAX_INPUT_SIZE (4096), the 256-byte `fgets` line buffer, and the
//! interaction between them.

mod common;
use common::assert_same;

fn tok_line(body: Vec<u8>) -> Vec<u8> {
    let mut input = b"6\n".to_vec();
    input.extend_from_slice(&body);
    input.extend_from_slice(b"\n\n3\n4\n7\n");
    input
}

fn analyze_line(body: Vec<u8>) -> Vec<u8> {
    let mut input = b"1\n".to_vec();
    input.extend_from_slice(&body);
    input.extend_from_slice(b"\n\n3\n4\n7\n");
    input
}

// --- MAX_TOKEN_LENGTH edges ------------------------------------------------

#[test]
fn identifier_length_around_max_token_length() {
    for n in [1usize, 253, 254, 255, 256, 257, 300, 512] {
        assert_same(&format!("ident-len-{n}"), &tok_line(vec![b'a'; n]));
        assert_same(&format!("ident-len-{n}-analyzed"), &analyze_line(vec![b'a'; n]));
    }
}

#[test]
fn number_length_around_max_token_length() {
    for n in [253usize, 254, 255, 256, 300] {
        assert_same(&format!("num-len-{n}"), &tok_line(vec![b'1'; n]));
    }
}

#[test]
fn plain_string_length_around_max_token_length() {
    for n in [251usize, 252, 253, 254, 255, 256, 300] {
        let mut body = vec![b'"'];
        body.extend(std::iter::repeat(b'x').take(n));
        body.push(b'"');
        assert_same(&format!("str-len-{n}"), &tok_line(body));
    }
}

#[test]
fn escape_sequences_can_push_the_string_buffer_one_past_its_loop_guard() {
    // scan_string()'s loop guard is `length < MAX_TOKEN_LENGTH - 2`, but the
    // escape branch appends two bytes, so `length` can reach 256 before
    // create_token() clamps it to 255.
    for n in [250usize, 251, 252, 253, 254, 255, 256, 260] {
        let mut body = vec![b'"'];
        for _ in 0..n {
            body.extend_from_slice(b"\\n");
        }
        body.push(b'"');
        assert_same(&format!("str-escapes-{n}"), &tok_line(body));
    }
}

#[test]
fn line_comment_length_around_max_token_length() {
    for n in [250usize, 252, 253, 254, 255, 256, 300] {
        let mut body = b"//".to_vec();
        body.extend(std::iter::repeat(b'z').take(n));
        assert_same(&format!("linecomment-len-{n}"), &tok_line(body));
    }
}

#[test]
fn block_comment_length_around_max_token_length() {
    for n in [250usize, 251, 252, 253, 254, 255, 300] {
        let mut body = b"/*".to_vec();
        body.extend(std::iter::repeat(b'z').take(n));
        body.extend_from_slice(b"*/");
        assert_same(&format!("blockcomment-len-{n}"), &tok_line(body));
    }
}

#[test]
fn block_comment_star_pair_at_the_buffer_edge() {
    // The `*` branch also appends two bytes past the `length < 254` guard.
    for n in [249usize, 250, 251, 252, 253] {
        let mut body = b"/*".to_vec();
        body.extend(std::iter::repeat(b'z').take(n));
        body.extend_from_slice(b"**/tail");
        assert_same(&format!("blockcomment-star-{n}"), &tok_line(body));
    }
}

#[test]
fn a_long_token_is_also_tracked_as_a_common_word() {
    // track_word() copies at most MAX_TOKEN_LENGTH - 1 bytes.
    let mut input = b"1\n".to_vec();
    input.extend(std::iter::repeat(b'q').take(300));
    input.push(b' ');
    input.extend(std::iter::repeat(b'q').take(300));
    input.extend_from_slice(b"\n\n3\n7\n");
    assert_same("track-long-word", &input);
}

// --- the 256-byte fgets buffer --------------------------------------------

#[test]
fn text_lines_around_the_fgets_buffer_size() {
    // fgets keeps at most 255 bytes, so a 255-byte line arrives without its
    // newline and the *next* read returns just "\n" -- which the C treats as
    // the empty line that ends the block.
    for n in [253usize, 254, 255, 256, 257, 300, 509, 510, 511, 512, 513] {
        assert_same(&format!("fgets-line-{n}"), &tok_line(vec![b'q'; n]));
    }
}

#[test]
fn a_255_byte_line_ends_the_input_block_early() {
    // Explicit form of the quirk above: the second line is never read as text.
    let mut input = b"1\n".to_vec();
    input.extend(std::iter::repeat(b'a').take(255));
    input.extend_from_slice(b"\nSECOND\n\n3\n7\n");
    assert_same("fgets-255-splits", &input);
}

// --- MAX_INPUT_SIZE (4096) edges ------------------------------------------

#[test]
fn accumulated_text_around_max_input_size() {
    // strncat is bounded by MAX_INPUT_SIZE - strlen(text) - 1, so the buffer
    // saturates at 4095 bytes and later lines are silently dropped.
    for n in [4093usize, 4094, 4095, 4096, 4097, 5000] {
        let mut input = b"1\n".to_vec();
        // 200-byte chunks keep every fgets call well under 255 bytes.
        let mut written = 0;
        while written < n {
            let chunk = std::cmp::min(200, n - written);
            input.extend(std::iter::repeat(b'a').take(chunk));
            input.push(b'\n');
            written += chunk;
        }
        input.extend_from_slice(b"\n3\n4\n7\n");
        assert_same(&format!("maxinput-{n}"), &input);
    }
}

#[test]
fn a_saturated_input_buffer_still_reads_and_discards_the_rest() {
    let mut input = b"6\n".to_vec();
    for i in 0..60 {
        input.extend_from_slice(format!("tok{i} ").as_bytes());
        input.extend(std::iter::repeat(b'p').take(100));
        input.push(b'\n');
    }
    input.extend_from_slice(b"TAIL_MARKER\n\n3\n7\n");
    assert_same("maxinput-saturated", &input);
}

#[test]
fn many_short_lines_fill_the_buffer_and_the_line_counter() {
    let mut input = b"1\n".to_vec();
    for i in 0..1500 {
        input.extend_from_slice(format!("l{i}\n").as_bytes());
    }
    input.extend_from_slice(b"\n3\n4\n7\n");
    assert_same("maxinput-many-lines", &input);
}

// --- full-session combinations -------------------------------------------

#[test]
fn a_long_session_touching_every_menu_choice() {
    assert_same(
        "session-all-choices",
        b"1\nint main(void) { return 0; }\n\n\
          6\nif (x <= y) { /* c */ z++; }\n\n\
          3\n4\n5\nx\n\
          2\n/dev/null\n\
          3\n4\n\
          0\nbogus\n\n\
          7\n",
    );
}

#[test]
fn interleaved_analyses_keep_the_static_state_growing() {
    let mut input = Vec::new();
    for i in 0..12 {
        input.extend_from_slice(b"1\n");
        input.extend_from_slice(format!("word{i} if else + ; // c\n").as_bytes());
        input.extend_from_slice(b"\n3\n4\n");
    }
    input.extend_from_slice(b"7\n");
    assert_same("session-interleaved", &input);
}

#[test]
fn deterministic_pseudorandom_soak() {
    // A small xorshift keeps this reproducible without a dependency. Each
    // case mixes menu choices with payload bytes drawn from the interesting
    // alphabet (quotes, slashes, operators, NULs, high bytes).
    const ALPHABET: &[u8] = b"abcXYZ_019 \t\"'\\/*+-=<>!&|^~?:(){}[];,.%#@\x00\x0b\x0c\r\x80\xff";
    const CHOICES: &[&[u8]] = &[
        b"1\n", b"3\n", b"4\n", b"5\n", b"6\n", b"0\n", b"x\n", b"\n", b"-3\n", b"8\n",
    ];

    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for case in 0..120u32 {
        let mut input = Vec::new();
        let rounds = 1 + (next() % 5) as usize;
        for _ in 0..rounds {
            input.extend_from_slice(CHOICES[(next() % CHOICES.len() as u64) as usize]);
            let lines = (next() % 4) as usize;
            for _ in 0..lines {
                let len = (next() % 60) as usize;
                for _ in 0..len {
                    input.push(ALPHABET[(next() % ALPHABET.len() as u64) as usize]);
                }
                input.push(b'\n');
            }
            input.push(b'\n');
        }
        if next() % 5 == 0 {
            while input.last() == Some(&b'\n') {
                input.pop();
            }
        }
        assert_same(&format!("soak-{case}"), &input);
    }
}
