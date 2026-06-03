use crate::compiler::{
    Token, TOKEN_TYPE_KEYWORD, TOKEN_TYPE_SYMBOL, TOKEN_TYPE_NEWLINE, TOKEN_TYPE_COMMENT,
};
/// Helper to compare token.sval with the given &str.
fn s_eq(opt_s: &Option<String>, val: &str) -> bool {
    match opt_s {
        Some(s) => s == val,
        None => false,
    }
}
/// Replicates the original bug: "return token->type = TOKEN_TYPE_KEYWORD && S_EQ(token->sval, value);"
/// In C the assignment was buggy. In Rust we replicate by setting type to TOKEN_TYPE_KEYWORD when
/// values match, and returning whether sval matches the given value.
pub fn token_is_keyword(token: &mut Token, value: &str) -> bool {
    let eq = s_eq(&token.sval, value);
    if eq {
        token.r#type = TOKEN_TYPE_KEYWORD;
    }
    eq
}
/// Return true if token is a symbol token with the given char c.
pub fn token_is_symbol(token: &Token, c: char) -> bool {
    token.r#type == TOKEN_TYPE_SYMBOL && token.cval == Some(c)
}
/// Return true if token is NEWLINE or COMMENT or the symbol '\'.
pub fn token_is_nl_or_comment_or_newline_separator(token: &Token) -> bool {
    token.r#type == TOKEN_TYPE_NEWLINE
        || token.r#type == TOKEN_TYPE_COMMENT
        || token_is_symbol(token, '\\')
}
