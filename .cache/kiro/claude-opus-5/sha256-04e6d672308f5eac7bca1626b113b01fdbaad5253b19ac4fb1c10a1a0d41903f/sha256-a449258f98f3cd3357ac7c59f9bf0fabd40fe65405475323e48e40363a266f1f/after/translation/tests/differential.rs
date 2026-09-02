//! Differential tests: run the C reference binary and the Rust binary as
//! subprocesses with identical argv/stdin and require byte-identical stdout,
//! byte-identical stderr and an identical exit status.
//!
//! The cases below are derived from a branch-by-branch reading of
//! `c_src/src/luggage.c`. Each test names the branch(es) it exercises.

mod harness;
use harness::{assert_same, c_bin, check, run_with_dropped_stdout, rust_bin, WILD};

// ---------------------------------------------------------------------------
// main(): argument-count validation (argc != 5 -> stderr + exit 1)
// ---------------------------------------------------------------------------

#[test]
fn argc_too_few_and_too_many() {
    let record = b"10 BAG1 FL1 JFK LAX note\n";
    // argc = 1
    assert_same("argc=1", &[], record);
    // argc = 2..4
    assert_same("argc=2", &[b"-"], record);
    assert_same("argc=3", &[b"-", b"-"], record);
    assert_same("argc=4", &[b"-", b"-", b"-"], record);
    // argc = 6, 7
    assert_same("argc=6", &[b"-", b"-", b"-", b"-", b"-"], record);
    assert_same("argc=7", &[b"-", b"-", b"-", b"-", b"-", b"-"], record);
}

#[test]
fn argc_error_ignores_stdin_entirely() {
    // The error path exits before reading stdin at all.
    assert_same("argc-err-empty-stdin", &[b"only-one"], b"");
    assert_same("argc-err-junk-stdin", &[b"a", b"b"], b"total garbage \x00\xff\n");
}

#[test]
fn argc_exactly_five_is_accepted() {
    check("argc=5", b"10 BAG1 FL1 JFK LAX note\n");
}

// ---------------------------------------------------------------------------
// Empty / whitespace-only input: the read loop breaks immediately and
// printMatchingDirectives() is called with first_directive == NULL.
// ---------------------------------------------------------------------------

#[test]
fn empty_input() {
    check("empty", b"");
}

#[test]
fn whitespace_only_input() {
    check("single-newline", b"\n");
    check("many-newlines", b"\n\n\n\n");
    check("spaces", b"    ");
    check("mixed-ws", b" \t\n\x0b\x0c\r \n");
    check("ws-no-eol", b"   ");
}

// ---------------------------------------------------------------------------
// Happy path: one record, then the maximum-width variants.
// ---------------------------------------------------------------------------

#[test]
fn single_record_with_comment() {
    // `%3[A-Z]` for the arrival is not followed by a whitespace directive, so
    // `%80[^\n]` keeps the separating blank and the output has two spaces
    // before the comment.
    check("one-record", b"100 LUG12345 FL1234 JFK LAX handle with care\n");
}

#[test]
fn single_record_without_comment() {
    // `%80[^\n]` fails to match at '\n' (matching failure, not EOF), so the
    // record is still stored with an empty comment -> trailing space.
    check("no-comment", b"100 LUG12345 FL1234 JFK LAX\n");
}

#[test]
fn maximum_field_widths() {
    check("max-widths", b"4294967295 ABCDEFGH ABCDEF ABC XYZ 12345678901234567890123456789012345678901234567890123456789012345678901234567890\n");
}

#[test]
fn fields_one_over_their_maximum_width() {
    // Each conversion stops at its width, and the leftover characters are
    // re-parsed by the *following* conversion.
    check("lug-9-chars", b"5 123456789 FL1 JFK LAX c\n");
    check("flight-7-chars", b"5 LUG1 1234567 JFK LAX c\n");
    check("departure-4-chars", b"5 LUG1 FL1 JFKX LAX c\n");
    check("arrival-4-chars", b"5 LUG1 FL1 JFK LAXX c\n");
    check("no-separators-at-all", b"5 LUG1 FL1 JFKLAXMIA c\n");
    check("all-fields-overlong", b"5 ABCDEFGHIJKLMNOP QRSTUVWXYZ JFK LAX c\n");
}

#[test]
fn comment_longer_than_eighty_characters() {
    // The first 80 bytes become the comment; the rest stays in the stream and
    // is re-parsed as the next record's timestamp.
    let mut input = b"5 LUG1 FL1 JFK LAX ".to_vec();
    input.extend_from_slice(&[b'9'; 100]);
    input.push(b'\n');
    check("comment-100-chars", &input);

    let mut exact = b"5 LUG1 FL1 JFK LAX ".to_vec();
    exact.extend_from_slice(&[b'x'; 79]);
    exact.push(b'\n');
    check("comment-exactly-80-incl-space", &exact);
}

// ---------------------------------------------------------------------------
// `%d ` -> unsigned int: signedness, truncation and overflow.
// ---------------------------------------------------------------------------

#[test]
fn timestamp_signedness_and_truncation() {
    check("ts-zero", b"0 A A JFK LAX c\n");
    check("ts-plus-sign", b"+42 A A JFK LAX c\n");
    check("ts-negative", b"-5 A A JFK LAX c\n");
    check("ts-int-min", b"-2147483648 A A JFK LAX c\n");
    check("ts-int-max", b"2147483647 A A JFK LAX c\n");
    check("ts-int-max-plus-1", b"2147483648 A A JFK LAX c\n");
    check("ts-u32-max", b"4294967295 A A JFK LAX c\n");
    check("ts-2-pow-32", b"4294967296 A A JFK LAX c\n");
    check("ts-2-pow-32-plus-7", b"4294967303 A A JFK LAX c\n");
    check("ts-leading-zeros", b"0000000000000005 A A JFK LAX c\n");
    check("ts-not-octal", b"010 A A JFK LAX c\n");
}

#[test]
fn timestamp_overflow_clamps_like_strtol() {
    check("ts-overflow-pos", b"99999999999999999999 A A JFK LAX c\n");
    check("ts-overflow-neg", b"-99999999999999999999 A A JFK LAX c\n");
    check("ts-long-max", b"9223372036854775807 A A JFK LAX c\n");
    check("ts-long-max-plus-1", b"9223372036854775808 A A JFK LAX c\n");
    check("ts-long-min", b"-9223372036854775808 A A JFK LAX c\n");
    check("ts-long-min-minus-1", b"-9223372036854775809 A A JFK LAX c\n");

    let mut huge = vec![b'9'; 10_000];
    huge.extend_from_slice(b" A A JFK LAX c\n");
    check("ts-10000-digits", &huge);

    let mut huge_neg = vec![b'-'];
    huge_neg.extend_from_slice(&[b'7'; 500]);
    huge_neg.extend_from_slice(b" A A JFK LAX c\n");
    check("ts-500-digits-negative", &huge_neg);
}

// ---------------------------------------------------------------------------
// scanf() returning EOF -> `break`. Each of the four scanf() calls has its own
// EOF exit, and the record in flight is discarded.
// ---------------------------------------------------------------------------

#[test]
fn eof_break_in_each_scanf() {
    // scanf("%d ") sees EOF straight away.
    check("eof-before-ts", b"10 A A JFK LAX c\n");
    // scanf("%8[..] %6[..] ") sees EOF: the timestamp was consumed, no record.
    check("eof-after-ts", b"55");
    check("eof-after-ts-space", b"55 ");
    check("eof-after-ts-newline", b"55\n");
    // The flight id hits EOF: scanf still returns 1, so no break here, but the
    // next scanf then breaks.
    check("eof-after-lug", b"55 LUG1");
    // scanf("%3[A-Z] %3[A-Z]") sees EOF.
    check("eof-after-flight", b"55 LUG1 FL1");
    // The arrival hits EOF (scanf returns 1), then %80[^\n] breaks.
    check("eof-after-departure", b"55 LUG1 FL1 JFK");
    check("eof-mid-departure", b"55 LUG1 FL1 JF");
    check("eof-mid-arrival", b"55 LUG1 FL1 JFK LA");
    // scanf("%80[^\n]") sees EOF -> the otherwise complete record is dropped.
    check("eof-after-arrival-complete", b"55 LUG1 FL1 JFK LAX");
    // ... but a comment without a trailing newline is a successful match, so
    // the record survives.
    check("no-trailing-newline-with-comment", b"55 LUG1 FL1 JFK LAX hi");
}

#[test]
fn last_record_truncated_after_a_complete_one() {
    check(
        "complete-then-truncated",
        b"10 BAG1 FL1 JFK LAX first\n20 BAG2 FL2 SFO SEA",
    );
    check(
        "two-complete-no-final-newline",
        b"10 BAG1 FL1 JFK LAX first\n20 BAG2 FL2 SFO SEA second",
    );
}

// ---------------------------------------------------------------------------
// scanf() *matching* failures (return value 0 or 1, never EOF): the C code does
// not break, and the untouched destination buffers keep their previous
// contents. On the very first iteration those contents are whatever main's
// stack frame happened to hold.
// ---------------------------------------------------------------------------

#[test]
fn matching_failure_on_first_iteration_uses_uninitialised_buffers() {
    // %d fails on 'a'; %8[A-Z0-9], %6[A-Z0-9], %3[A-Z], %3[A-Z] all fail too;
    // %80[^\n] then swallows the whole line.
    check("all-lowercase", b"abc\n");
    check("lowercase-sentence", b"hello world\n");
    // %d fails, everything else fails, comment is punctuation.
    check("punctuation-only", b"!!! ???\n");
    // A bare sign followed by a non-digit is also a matching failure for %d.
    check("sign-then-letter", b"-x\n");
    check("sign-then-space", b"- 5 LUG1 FL1 JFK LAX c\n");
    check("plus-then-letter", b"+q\n");
    // %d succeeds, the luggage id fails.
    check("ts-ok-lug-fails", b"5 zzz\n");
    // %d succeeds, luggage id succeeds, flight id fails.
    check("lug-ok-flight-fails", b"5 LUG1 zzz\n");
    // ... departure fails.
    check("flight-ok-departure-fails", b"5 LUG1 FL1 zzz\n");
    // ... arrival fails.
    check("departure-ok-arrival-fails", b"5 LUG1 FL1 JFK zzz\n");
    // %d stops at the first non-digit, which then feeds the later conversions.
    check("digits-then-letter", b"5x LUG1 FL1 JFK LAX c\n");
}

#[test]
fn matching_failure_reuses_the_previous_iterations_values() {
    // Iteration 2 fails on every string conversion, so it inherits iteration
    // 1's luggage id, flight id, departure and arrival.
    check("stale-all", b"5 LUG1 FL1 JFK LAX c\n7 zzz\n");
    check("stale-flight-onwards", b"5 LUG1 FL1 JFK LAX c\n7 BAG9 zzz\n");
    check("stale-departure-onwards", b"5 LUG1 FL1 JFK LAX c\n7 BAG9 FL9 zzz\n");
    check("stale-arrival-only", b"5 LUG1 FL1 JFK LAX c\n7 BAG9 FL9 SFO zzz\n");
    // Stale timestamp: %d fails in iteration 2.
    check("stale-timestamp", b"5 LUG1 FL1 JFK LAX c\nq BAG9 FL9 SFO SEA d\n");
    // Three iterations of decay.
    check("stale-cascade", b"5 LUG1 FL1 JFK LAX c\n7 zzz\n9 yyy\n");
}

#[test]
fn comment_matching_failure_leaves_it_empty() {
    // `comments[0] = 0` at the top of the loop means a failed %80[^\n] yields an
    // empty comment even though the previous iteration had one.
    check("comment-cleared", b"5 LUG1 FL1 JFK LAX long comment\n7 BAG9 FL9 SFO SEA\n");
    check("comment-single-space", b"5 A A JFK LAX \n");
    check("comment-only-spaces", b"5 A A JFK LAX      \n");
    check("comment-tab", b"5\tA\tA\tJFK\tLAX\tc\n");
}

// ---------------------------------------------------------------------------
// strcpy() truncates at the first NUL, but %80[^\n] happily stores one.
// ---------------------------------------------------------------------------

#[test]
fn nul_bytes_in_the_comment_truncate_the_stored_string() {
    check("nul-mid-comment", b"5 LUG1 FL1 JFK LAX ab\x00cd\n");
    check("nul-first-comment-byte", b"5 LUG1 FL1 JFK LAX \x00abc\n");
    check("nul-only", b"5 LUG1 FL1 JFK LAX \x00\n");
    // The NUL still counts against the 80-char width, so the stream position
    // after the conversion depends on the bytes *after* the NUL as well.
    let mut wide = b"5 LUG1 FL1 JFK LAX a\x00".to_vec();
    wide.extend_from_slice(&[b'b'; 100]);
    wide.push(b'\n');
    check("nul-then-100-bytes", &wide);
    check("several-nuls", b"5 LUG1 FL1 JFK LAX \x00\x00\x00x\n");
    check("nul-then-newline-then-record", b"5 A A JFK LAX x\x00y\n9 B B SFO SEA z\n");
}

// ---------------------------------------------------------------------------
// Non-ASCII and control bytes.
// ---------------------------------------------------------------------------

#[test]
fn high_and_control_bytes_in_comments() {
    check("utf8-comment", b"5 LUG1 FL1 JFK LAX caf\xc3\xa9\n");
    check("high-bytes", b"5 LUG1 FL1 JFK LAX \x80\xfe\xff\n");
    check("carriage-return", b"5 LUG1 FL1 JFK LAX x\r\n");
    check("cr-only-line-ends", b"5 LUG1 FL1 JFK LAX x\r9 B B SFO SEA y\r");
    check("control-bytes", b"5 LUG1 FL1 JFK LAX \x01\x02\x03\x7f\n");
}

// ---------------------------------------------------------------------------
// addRoutingDirectiveToList(): tail append, head insert, middle insert and the
// stable ordering of equal timestamps.
// ---------------------------------------------------------------------------

#[test]
fn insertion_sort_orders_by_timestamp() {
    check("ascending", b"10 A F JFK LAX a\n20 B F JFK LAX b\n30 C F JFK LAX c\n");
    check("descending", b"30 A F JFK LAX a\n20 B F JFK LAX b\n10 C F JFK LAX c\n");
    check("middle-insert", b"30 A F JFK LAX a\n10 B F JFK LAX b\n20 C F JFK LAX c\n");
    check("zero-first", b"0 A F JFK LAX a\n0 B F JFK LAX b\n");
    check(
        "shuffled",
        b"50 A F JFK LAX a\n10 B F JFK LAX b\n40 C F JFK LAX c\n20 D F JFK LAX d\n30 E F JFK LAX e\n",
    );
}

#[test]
fn equal_timestamps_keep_insertion_order() {
    check(
        "equal-timestamps",
        b"10 A F JFK LAX first\n10 B F JFK LAX second\n10 C F JFK LAX third\n10 D F JFK LAX fourth\n",
    );
    // A zero timestamp equals the list head's timestamp, which the C code never
    // prints but does compare against.
    check("all-zero-timestamps", b"0 A F JFK LAX a\n0 B F JFK LAX b\n0 C F JFK LAX c\n");
}

// ---------------------------------------------------------------------------
// supersedes()/superseded(): stops at the *first* later directive with the same
// luggage id and only reports "superseded" when the departure also matches.
// ---------------------------------------------------------------------------

#[test]
fn superseded_when_a_later_directive_repeats_id_and_departure() {
    check("superseded-simple", b"10 BAG1 F1 JFK LAX old\n20 BAG1 F1 JFK LAX new\n");
    check(
        "superseded-chain",
        b"10 BAG1 F1 JFK LAX a\n20 BAG1 F1 JFK LAX b\n30 BAG1 F1 JFK LAX c\n",
    );
}

#[test]
fn not_superseded_when_the_departure_differs() {
    check("different-departure", b"10 BAG1 F1 JFK LAX old\n20 BAG1 F1 SFO LAX new\n");
    // The search stops at the first id match even though a *later* directive
    // would have matched the departure -- that is the quirk to preserve.
    check(
        "search-stops-at-first-id-match",
        b"10 BAG1 F1 JFK LAX a\n20 BAG1 F1 SFO LAX b\n30 BAG1 F1 JFK LAX c\n",
    );
    check(
        "search-stops-at-first-id-match-longer",
        b"10 BAG1 F1 JFK LAX a\n20 BAG1 F1 SFO LAX b\n30 BAG1 F1 SFO LAX c\n40 BAG1 F1 JFK LAX d\n",
    );
}

#[test]
fn unrelated_ids_are_skipped_during_the_supersede_search() {
    check(
        "interleaved-ids",
        b"10 BAG1 F1 JFK LAX a\n15 BAG2 F1 SFO LAX b\n20 BAG1 F1 JFK LAX c\n",
    );
    check(
        "no-later-match",
        b"10 BAG1 F1 JFK LAX a\n20 BAG2 F1 JFK LAX b\n30 BAG3 F1 JFK LAX c\n",
    );
    // Ordering matters: the supersede search walks the *sorted* list, not the
    // input order.
    check(
        "reordered-supersede",
        b"20 BAG1 F1 JFK LAX later\n10 BAG1 F1 SFO LAX earlier\n",
    );
}

#[test]
fn empty_and_uninitialised_fields_participate_in_the_supersede_search() {
    // Iteration 2's luggage id and departure are inherited, which makes it
    // supersede iteration 1.
    check("stale-supersedes", b"10 BAG1 F1 JFK LAX a\n20 zzz\n");
    // Two records that both fail every string conversion share the same
    // (uninitialised) id and departure.
    check("both-uninitialised", b"aaa\nbbb\n");
}

// ---------------------------------------------------------------------------
// matches(): a leading '-' is a wildcard, otherwise strcmp equality.
// ---------------------------------------------------------------------------

const TWO: &[u8] = b"10 BAG1 FL1 JFK LAX first\n20 BAG2 FL2 SFO SEA second\n";

#[test]
fn wildcard_argument_matches_everything() {
    assert_same("wild-all", WILD, TWO);
    assert_same("wild-dash-prefix", &[b"-abc", b"-1", b"--", b"-\xff"], TWO);
    assert_same("wild-single-dash-mixed", &[b"BAG1", b"-", b"-", b"-"], TWO);
}

#[test]
fn exact_arguments_filter_each_field() {
    assert_same("filter-luggage", &[b"BAG1", b"-", b"-", b"-"], TWO);
    assert_same("filter-flight", &[b"-", b"FL2", b"-", b"-"], TWO);
    assert_same("filter-departure", &[b"-", b"-", b"JFK", b"-"], TWO);
    assert_same("filter-arrival", &[b"-", b"-", b"-", b"SEA"], TWO);
    assert_same("filter-all-four", &[b"BAG2", b"FL2", b"SFO", b"SEA"], TWO);
    assert_same("filter-conflicting", &[b"BAG1", b"FL2", b"-", b"-"], TWO);
}

#[test]
fn non_matching_arguments_print_nothing() {
    assert_same("filter-miss-luggage", &[b"NOPE", b"-", b"-", b"-"], TWO);
    assert_same("filter-prefix-only", &[b"BAG", b"-", b"-", b"-"], TWO);
    assert_same("filter-superstring", &[b"BAG11", b"-", b"-", b"-"], TWO);
    assert_same("filter-lowercase", &[b"bag1", b"-", b"-", b"-"], TWO);
}

#[test]
fn empty_argument_is_not_a_wildcard() {
    // expected[0] is '\0', not '-', so it falls through to strcmp("", actual).
    assert_same("empty-args-vs-records", &[b"", b"", b"", b""], TWO);
    // An empty string does match a field that was never assigned.
    assert_same("empty-arg-matches-empty-flight", &[b"-", b"", b"-", b"-"], b"5 LUG1 zzz\n");
    assert_same(
        "empty-args-match-uninitialised-tail",
        &[b"-", b"", b"", b""],
        b"5 zzz\n",
    );
}

#[test]
fn arguments_with_odd_bytes() {
    assert_same("arg-high-bytes", &[b"\xff\xfe", b"-", b"-", b"-"], TWO);
    assert_same("arg-utf8", &[b"caf\xc3\xa9", b"-", b"-", b"-"], TWO);
    assert_same("arg-space", &[b"BAG1 ", b"-", b"-", b"-"], TWO);
    // Matches the uninitialised luggage id of the reference build.
    assert_same("arg-ctrl-c", &[b"\x03", b"-", b"-", b"-"], b"abc\n");
}

// ---------------------------------------------------------------------------
// scanf() reads across newlines: a record may be spread over several lines.
// ---------------------------------------------------------------------------

#[test]
fn records_span_newlines() {
    check("one-field-per-line", b"5\nLUG1\nFL1\nJFK\nLAX\n");
    check("blank-lines-between-fields", b"5\n\n\nLUG1\n\n\nFL1\n\n\nJFK\n\n\nLAX\n");
    check("leading-blank-lines", b"\n\n\n5 A A JFK LAX c\n\n\n");
    check("everything-on-one-line", b"5 A A JFK LAX c 9 B B SFO SEA d\n");
    check("crlf-separated", b"5 A A JFK LAX c\r\n9 B B SFO SEA d\r\n");
    check(
        "vertical-tab-and-formfeed",
        b"5\x0bLUG1\x0cFL1\x0bJFK\x0cLAX c\n",
    );
}

// ---------------------------------------------------------------------------
// Volume: exercises the recursive insertion and the O(n^2) supersede scan at a
// size both programs handle comfortably.
// ---------------------------------------------------------------------------

#[test]
fn many_records() {
    let mut ascending = Vec::new();
    let mut descending = Vec::new();
    let mut duplicates = Vec::new();
    for i in 0..400u32 {
        ascending.extend_from_slice(format!("{} BAG{:05} FL1 JFK LAX c{}\n", i + 1, i, i).as_bytes());
        descending
            .extend_from_slice(format!("{} BAG{:05} FL1 JFK LAX c{}\n", 400 - i, i, i).as_bytes());
        duplicates.extend_from_slice(
            format!("{} BAG{:02} FL1 {} LAX c{}\n", i % 7, i % 13, if i % 2 == 0 { "JFK" } else { "SFO" }, i)
                .as_bytes(),
        );
    }
    check("400-ascending", &ascending);
    check("400-descending", &descending);
    check("400-duplicate-ids", &duplicates);
    assert_same("400-filtered", &[b"BAG00042", b"-", b"-", b"-"], &ascending);
}

// ---------------------------------------------------------------------------
// Exit status when stdout goes away: the C program is killed by SIGPIPE, so the
// Rust program must not silently swallow EPIPE and exit 0.
// ---------------------------------------------------------------------------

#[test]
fn dying_on_a_closed_stdout_pipe() {
    // ~170 KiB of output, well past the 64 KiB pipe buffer.
    let mut input = Vec::new();
    for i in 0..5000u32 {
        input.extend_from_slice(format!("{} BAG{:05} FL1 JFK LAX c\n", i + 1, i).as_bytes());
    }
    for attempt in 0..3 {
        let c = run_with_dropped_stdout(c_bin(), WILD, &input);
        let r = run_with_dropped_stdout(rust_bin(), WILD, &input);
        assert_eq!(
            c, r,
            "closed-stdout exit status differs on attempt {attempt}: C={c:?} Rust={r:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Randomised differential sweep with fixed seeds, so it is reproducible and
// never flaky. Uses a small xorshift PRNG rather than a dependency.
// ---------------------------------------------------------------------------
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

const ALPHABET: &[u8] =
    b"ABCXYZ0189abcz -\t\n\r\x0b\x0c\x00\x01\xff\x80+.!";

fn random_record(rng: &mut Rng) -> Vec<u8> {
    const TS: &[&[u8]] = &[
        b"0", b"1", b"5", b"10", b"20", b"4294967295", b"2147483648", b"-1",
        b"-2147483648", b"99999999999999999999", b"007", b"", b"-", b"+3", b"qq",
    ];
    const LUG: &[&[u8]] = &[b"BAG1", b"BAG2", b"A", b"12345678", b"ABCDEFGHIJ", b"zz", b""];
    const FL: &[&[u8]] = &[b"FL1", b"FL2", b"F", b"123456", b"ABCDEFGH", b"yy", b""];
    const DEP: &[&[u8]] = &[b"JFK", b"SFO", b"LAX", b"AB", b"ABCD", b"A", b"ww", b""];
    const ARR: &[&[u8]] = &[b"LAX", b"SEA", b"MIA", b"XY", b"WXYZ", b"Q", b"vv", b""];
    const CMT: &[&[u8]] = &[b"", b" ", b" hello there", b"\tx", b" \x00mid", b"  double"];
    const SEP: &[&[u8]] = &[b" ", b"  ", b"\t", b"\n", b" \n ", b"\x0b", b"\r\n"];

    let mut out = Vec::new();
    out.extend_from_slice(rng.pick(TS));
    out.extend_from_slice(rng.pick(SEP));
    out.extend_from_slice(rng.pick(LUG));
    out.extend_from_slice(rng.pick(SEP));
    out.extend_from_slice(rng.pick(FL));
    out.extend_from_slice(rng.pick(SEP));
    out.extend_from_slice(rng.pick(DEP));
    out.extend_from_slice(rng.pick(SEP));
    out.extend_from_slice(rng.pick(ARR));
    out.extend_from_slice(rng.pick(CMT));
    out
}

fn arg_sets() -> Vec<Vec<&'static [u8]>> {
    vec![
        vec![b"-", b"-", b"-", b"-"],
        vec![b"BAG1", b"-", b"-", b"-"],
        vec![b"-", b"FL1", b"-", b"-"],
        vec![b"-", b"-", b"JFK", b"-"],
        vec![b"-", b"-", b"-", b"LAX"],
        vec![b"BAG1", b"FL1", b"JFK", b"LAX"],
        vec![b"", b"", b"", b""],
        vec![b"-x", b"-y", b"-z", b"-w"],
        vec![b"A", b"F", b"AB", b"XY"],
        vec![b"\x03", b"-", b"-", b"-"],
    ]
}

#[test]
fn randomised_structured_records() {
    let sets = arg_sets();
    for seed in 1..=600u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let mut input = Vec::new();
        for _ in 0..rng.below(6) {
            input.extend_from_slice(&random_record(&mut rng));
            input.push(b'\n');
        }
        if rng.below(3) == 0 && !input.is_empty() {
            input.pop(); // drop the final newline
        }
        let args = rng.pick(&sets).clone();
        assert_same(&format!("rand-structured-{seed}"), &args, &input);
    }
}

#[test]
fn randomised_byte_soup() {
    let sets = arg_sets();
    for seed in 1..=600u64 {
        let mut rng = Rng(seed.wrapping_mul(0xD1B5_4A32_D192_ED03) | 1);
        let len = rng.below(120);
        let mut input = Vec::with_capacity(len);
        for _ in 0..len {
            input.push(*rng.pick(ALPHABET));
        }
        let args = rng.pick(&sets).clone();
        assert_same(&format!("rand-bytes-{seed}"), &args, &input);
    }
}

#[test]
fn randomised_token_streams() {
    let sets = arg_sets();
    const TOKENS: &[&[u8]] = &[
        b"5", b"-7", b"+0", b"4294967296", b"BAG1", b"FL1", b"JFK", b"LAX", b"abc",
        b"ABCDEFGHIJKL", b"123456789012", b"", b"-", b"!", b"\x00", b"\xff", b".",
    ];
    const SEP: &[&[u8]] = &[b" ", b"\n", b"\t", b"  ", b"\r", b"\n\n"];
    for seed in 1..=600u64 {
        let mut rng = Rng(seed.wrapping_mul(0xA076_1D64_78BD_642F) | 1);
        let mut input = Vec::new();
        for i in 0..rng.below(14) {
            if i > 0 {
                input.extend_from_slice(rng.pick(SEP));
            }
            input.extend_from_slice(rng.pick(TOKENS));
        }
        let args = rng.pick(&sets).clone();
        assert_same(&format!("rand-tokens-{seed}"), &args, &input);
    }
}
