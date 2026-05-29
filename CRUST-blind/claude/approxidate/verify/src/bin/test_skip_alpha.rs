#![allow(unused_imports)]
use approxidate::approxidate;

#[test]
fn test_skip_alpha_pure_alpha() {
    // "Mon" -> i starts at 0; do{i++}while(isalpha(date[i]));
    // i=1 (date[1]='o' alpha), i=2 (date[2]='n' alpha), i=3 (date[3]=0 not alpha) -> 3.
    assert_eq!(approxidate::skip_alpha("Mon"), 3);
}

#[test]
fn test_skip_alpha_followed_by_digit() {
    assert_eq!(approxidate::skip_alpha("Mon123"), 3);
}

#[test]
fn test_skip_alpha_followed_by_space() {
    assert_eq!(approxidate::skip_alpha("abc def"), 3);
}

#[test]
fn test_skip_alpha_single_alpha() {
    // do/while runs at least once: i becomes 1, then date[1] = 0 -> break -> return 1.
    assert_eq!(approxidate::skip_alpha("a"), 1);
}

#[test]
fn test_skip_alpha_long() {
    assert_eq!(approxidate::skip_alpha("January"), 7);
}

fn main() {}
