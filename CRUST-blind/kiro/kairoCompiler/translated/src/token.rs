use crate::compiler::{
    Token, TOKEN_TYPE_KEYWORD, TOKEN_TYPE_SYMBOL, TOKEN_TYPE_NEWLINE, TOKEN_TYPE_COMMENT,
};

fn s_eq(opt_s: &Option<String>, val: &str) -> bool {
    match opt_s {
        Some(s) => s == val,
        None => false,
    }
}

pub fn token_is_keyword(token: &mut Token, value: &str) -> bool {
    let eq = token.r#type == TOKEN_TYPE_KEYWORD && s_eq(&token.sval, value);
    token.r#type = if eq { 1 } else { 0 };
    eq
}

pub fn token_is_symbol(token: &Token, c: char) -> bool {
    token.r#type == TOKEN_TYPE_SYMBOL && token.cval == Some(c)
}

pub fn token_is_nl_or_comment_or_newline_separator(token: &Token) -> bool {
    token.r#type == TOKEN_TYPE_NEWLINE
        || token.r#type == TOKEN_TYPE_COMMENT
        || token_is_symbol(token, '\\')
}
