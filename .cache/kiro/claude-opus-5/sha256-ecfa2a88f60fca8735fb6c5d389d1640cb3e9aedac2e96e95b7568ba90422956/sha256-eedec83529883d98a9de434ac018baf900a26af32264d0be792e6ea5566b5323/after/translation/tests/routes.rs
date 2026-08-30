//! Case 2 (add route): the three `fgets` calls, the distance conversion, the
//! order of the lookups and of `add_edge`'s own checks.

mod harness;
use harness::{same, same_merged};

#[test]
fn add_route() {
    same("add_route", b"1\nA\n1\nB\n2\nA\nB\n100\n3\n8\n");
}

#[test]
fn add_route_eof_at_each_prompt() {
    same("route_eof_from", b"2\n");
    same("route_eof_to", b"2\nA\n");
    same("route_eof_distance", b"2\nA\nB\n");
    same("route_eof_from_no_newline", b"2\nA");
}

#[test]
fn invalid_distance() {
    same("invalid_distance", b"1\nA\n1\nB\n2\nA\nB\nxyz\n8\n");
    same("blank_distance", b"1\nA\n1\nB\n2\nA\nB\n\n8\n");
    same("sign_only_distance", b"1\nA\n1\nB\n2\nA\nB\n-\n8\n");
    // The distance is read and rejected before either city is looked up.
    same("invalid_distance_unknown_cities", b"2\nNope\nAlsoNope\nxyz\n8\n");
}

#[test]
fn distance_conversion_edges() {
    same("distance_leading_space", b"1\nA\n1\nB\n2\nA\nB\n   42abc\n8\n3\n");
    same("distance_zero", b"1\nA\n1\nB\n2\nA\nB\n0\n3\n8\n");
    same("distance_int_max", b"1\nA\n1\nB\n2\nA\nB\n2147483647\n3\n8\n");
    // Truncation of a long into an int: 4294967296 becomes 0, 2^31 becomes
    // INT_MIN and is rejected as negative.
    same("distance_2_pow_32", b"1\nA\n1\nB\n2\nA\nB\n4294967296\n3\n8\n");
    same("distance_2_pow_31", b"1\nA\n1\nB\n2\nA\nB\n2147483648\n3\n8\n");
    same("distance_14_digits", b"1\nA\n1\nB\n2\nA\nB\n99999999999999\n3\n8\n");
    same("distance_over_long_max", b"1\nA\n1\nB\n2\nA\nB\n99999999999999999999\n3\n8\n");
    same("distance_under_long_min", b"1\nA\n1\nB\n2\nA\nB\n-99999999999999999999\n3\n8\n");
}

#[test]
fn negative_distance() {
    same("negative_distance", b"1\nA\n1\nB\n2\nA\nB\n-5\n3\n8\n");
    same("negative_zero_distance", b"1\nA\n1\nB\n2\nA\nB\n-0\n3\n8\n");
}

#[test]
fn unknown_cities() {
    // The from city is reported first even when both are unknown.
    same("route_from_unknown", b"2\nX\nY\n5\n8\n");
    same("route_to_unknown", b"1\nA\n2\nA\nZ\n5\n8\n");
    same("route_from_unknown_to_known", b"1\nB\n2\nA\nB\n5\n8\n");
    // A name too long to have been stored intact never matches.
    let mut input = Vec::new();
    input.extend_from_slice(b"1\n");
    input.extend_from_slice(&[b'L'; 70]);
    input.extend_from_slice(b"\n2\n");
    input.extend_from_slice(&[b'L'; 70]);
    input.extend_from_slice(b"\n");
    input.extend_from_slice(&[b'L'; 70]);
    input.extend_from_slice(b"\n5\n8\n");
    same("route_long_name_never_matches", &input);
}

#[test]
fn empty_city_name_route() {
    same("route_empty_names", b"1\n\n2\n\n\n5\n3\n8\n");
}

#[test]
fn duplicate_route() {
    same("duplicate_route", b"1\nA\n1\nB\n2\nA\nB\n5\n2\nA\nB\n7\n3\n8\n");
    // A duplicate in the other direction is a different edge.
    same("reverse_route", b"1\nA\n1\nB\n2\nA\nB\n5\n2\nB\nA\n7\n3\n8\n");
}

#[test]
fn self_route() {
    same("self_route", b"1\nA\n2\nA\nA\n0\n3\n2\nA\nA\n9\n8\n");
}

#[test]
fn max_edges() {
    // MAX_EDGES = 10; the 11th edge is refused.
    let mut input = Vec::new();
    for i in 0..12 {
        input.extend_from_slice(format!("1\nP{i}\n").as_bytes());
    }
    for i in 1..12 {
        input.extend_from_slice(format!("2\nP0\nP{i}\n{i}\n").as_bytes());
    }
    input.extend_from_slice(b"4\nP0\n8\n");
    same("max_edges", &input);
}

#[test]
fn max_edges_is_checked_before_the_distance_sign() {
    // With 10 edges already present, a negative distance still reports the
    // edge-count error, because that check comes first.
    let mut input = Vec::new();
    for i in 0..11 {
        input.extend_from_slice(format!("1\nP{i}\n").as_bytes());
    }
    for i in 1..11 {
        input.extend_from_slice(format!("2\nP0\nP{i}\n{i}\n").as_bytes());
    }
    input.extend_from_slice(b"2\nP0\nP1\n-3\n2\nP0\nP10\n-3\n4\nP0\n8\n");
    same("max_edges_before_negative", &input);
}

#[test]
fn duplicate_check_comes_after_the_distance_sign() {
    // An existing edge plus a negative distance reports the negative distance.
    same(
        "negative_before_duplicate",
        b"1\nA\n1\nB\n2\nA\nB\n5\n2\nA\nB\n-1\n3\n8\n",
    );
}

#[test]
fn merged_streams_routes() {
    same_merged(
        "merged_routes",
        b"1\nA\n1\nB\n2\nA\nB\n-5\n2\nA\nB\n5\n2\nA\nB\n5\n2\nA\nZ\n5\n3\n8\n",
    );
}
