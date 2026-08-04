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
/// In C, that's an assignment of (TOKEN_TYPE_KEYWORD && S_EQ(...)) into token->type.
/// Since TOKEN_TYPE_KEYWORD is non-zero, that boils down to assigning S_EQ(...) (1 or 0) to token->type
/// and returning that same value.
pub fn token_is_keyword(token: &mut Token, value: &str) -> bool {
    let eq = s_eq(&token.sval, value);
    let assigned: i32 = if eq { 1 } else { 0 };
    token.r#type = assigned;
    assigned != 0
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

// Suppress unused warning for TOKEN_TYPE_KEYWORD if not used directly
#[allow(dead_code)]
const _UNUSED: i32 = TOKEN_TYPE_KEYWORD;
