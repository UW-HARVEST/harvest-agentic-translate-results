use simple_lang::settings::{
    DEBUG, MAX_INSTR_IN_STATEMENT, MAX_SOURCE_LENGTH, MAX_STATEMENT, MAX_TOKEN, STAKE_LENGTH,
};

#[test]
fn test_max_token() {
    assert_eq!(MAX_TOKEN, 200);
}

#[test]
fn test_max_statement() {
    assert_eq!(MAX_STATEMENT, 100);
}

#[test]
fn test_max_instr_in_statement() {
    assert_eq!(MAX_INSTR_IN_STATEMENT, 100);
}

#[test]
fn test_stake_length() {
    assert_eq!(STAKE_LENGTH, 200);
}

#[test]
fn test_max_source_length() {
    assert_eq!(MAX_SOURCE_LENGTH, 2000);
}

#[test]
fn test_debug() {
    assert_eq!(DEBUG, 0);
}

fn main() {}
