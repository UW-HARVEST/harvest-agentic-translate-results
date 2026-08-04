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

/// Replicates the original C bug: "return token->type = TOKEN_TYPE_KEYWORD && S_EQ(token->sval, value);"
/// In C, this `=` (single equal) is an assignment of the bool result of && back to type, and the
/// result of the assignment is what's returned. The truthiness equates to "the sval matched".
pub fn token_is_keyword(token: &mut Token, value: &str) -> bool {
    let eq = s_eq(&token.sval, value);
    // Mimic the C assignment: token->type = (TOKEN_TYPE_KEYWORD && S_EQ(...))
    // In C, TOKEN_TYPE_KEYWORD (1) is truthy, so the result is just the value of S_EQ(...)
    // Then token->type is set to that result (0 or 1). Function returns the assigned value.
    let new_type = if eq { 1 } else { 0 };
    token.r#type = new_type;
    new_type != 0
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

// Suppress unused warning
#[allow(dead_code)]
const _: i32 = TOKEN_TYPE_KEYWORD;
