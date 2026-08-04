#![allow(unused_imports)]
use approxidate::approxidate;

#[test]
fn test_match_string_exact() {
    // exact match returns string length
    assert_eq!(approxidate::match_string("January", "January"), 7);
}

#[test]
fn test_match_string_partial() {
    // "Jan" is a prefix of "January"; the C loop terminates when *date = 0,
    // returning the number of chars matched from date.
    assert_eq!(approxidate::match_string("Jan", "January"), 3);
}

#[test]
fn test_match_string_longer_date() {
    // Date longer than format: stops on non-alnum non-matching char.
    assert_eq!(approxidate::match_string("January 5", "January"), 7);
}

#[test]
fn test_match_string_case_insensitive() {
    assert_eq!(approxidate::match_string("JANuary 5", "January"), 7);
}

#[test]
fn test_match_string_no_match() {
    // Mismatch on alnum char => 0.
    assert_eq!(approxidate::match_string("foo", "bar"), 0);
}

#[test]
fn test_match_string_partial_5char() {
    assert_eq!(approxidate::match_string("Janua", "January"), 5);
}

#[test]
fn test_match_string_empty_date() {
    assert_eq!(approxidate::match_string("", "abc"), 0);
}

#[test]
fn test_match_string_break_on_non_alnum() {
    // After 3 chars match, ' ' is non-alnum so break -> returns 3.
    assert_eq!(approxidate::match_string("Jan ", "January"), 3);
}

fn main() {}
