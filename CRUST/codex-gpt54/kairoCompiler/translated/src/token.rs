use crate::compiler::{
    Token, TOKEN_TYPE_COMMENT, TOKEN_TYPE_KEYWORD, TOKEN_TYPE_NEWLINE, TOKEN_TYPE_SYMBOL,
};

fn s_eq(opt_s: &Option<String>, val: &str) -> bool {
    opt_s.as_deref() == Some(val)
}

pub fn token_is_keyword(token: &mut Token, value: &str) -> bool {
    let is_keyword = s_eq(&token.sval, value);
    token.r#type = if is_keyword { TOKEN_TYPE_KEYWORD } else { 0 };
    is_keyword
}

pub fn token_is_symbol(token: &Token, c: char) -> bool {
    token.r#type == TOKEN_TYPE_SYMBOL && token.cval == Some(c)
}

pub fn token_is_nl_or_comment_or_newline_separator(token: &Token) -> bool {
    token.r#type == TOKEN_TYPE_NEWLINE
        || token.r#type == TOKEN_TYPE_COMMENT
        || token_is_symbol(token, '\\')
}
