use simple_lang::settings;

#[test]
fn test_settings_constants() {
    assert_eq!(settings::MAX_TOKEN, 200);
    assert_eq!(settings::MAX_STATEMENT, 100);
    assert_eq!(settings::MAX_INSTR_IN_STATEMENT, 100);
    assert_eq!(settings::STAKE_LENGTH, 200);
    assert_eq!(settings::MAX_SOURCE_LENGTH, 2000);
    assert_eq!(settings::DEBUG, 0);
    assert!(settings::SIMPLE_LANG_SETTINGS_H);
}

fn main() {}
