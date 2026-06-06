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
/// We do this in Rust by assigning token.r#type = if eq { 1 } else { 0 } and then returning eq.
pub fn token_is_keyword(token: &mut Token, value: &str) -> bool {
    let eq = s_eq(&token.sval, value);
    // The C bug is: token->type = (TOKEN_TYPE_KEYWORD && S_EQ(...))
    // (TOKEN_TYPE_KEYWORD && bool) is treated as 1 when both true, 0 otherwise.
    // TOKEN_TYPE_KEYWORD is 1 (truthy), so result depends on `eq`.
    token.r#type = if eq { 1 } else { 0 };
    let _ = TOKEN_TYPE_KEYWORD;
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
