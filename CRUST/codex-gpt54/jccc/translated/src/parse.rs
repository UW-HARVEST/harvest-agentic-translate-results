use crate::lex::Lexer;
use crate::cst::{
    BlockStatement, ConcreteFileTree, Expression, FunctionCall, FunctionDeclaration, NodeType,
    TopLevelDeclaration,
};
use crate::lex::{empty_token, lex};
use crate::list::{create_list, ladd_element};
use crate::token::{Token, TokenType};
use std::array;
use std::fs::File;

fn make_lexer(filename: &str) -> Option<Lexer> {
    let fp = File::open(filename).ok()?;
    Some(Lexer {
        fp: Some(fp),
        current_file: filename.to_string(),
        buffer: [0],
        position: 0,
        last_column: 0,
        column: 1,
        line: 1,
        unlexed: array::from_fn(|_| empty_token()),
        unlexed_count: 0,
    })
}

/// Parses a function declaration from the lexer into a FunctionDeclaration object.
pub fn parse_funcdecl(l: &mut Lexer, fd: &mut FunctionDeclaration) -> i32 {
    let mut ret_type = empty_token();
    let mut name = empty_token();
    let mut oparen = empty_token();
    let mut cparen = empty_token();

    if lex(l, &mut ret_type) != 0 || ret_type.token_type != TokenType::TT_INT {
        return -1;
    }
    if lex(l, &mut name) != 0 || name.token_type != TokenType::TT_IDENTIFIER {
        return -1;
    }
    if lex(l, &mut oparen) != 0 || oparen.token_type != TokenType::TT_OPAREN {
        return -1;
    }
    if lex(l, &mut cparen) != 0 || cparen.token_type != TokenType::TT_CPAREN {
        return -1;
    }

    fd.name = name.contents;
    parse_blockstmt(l, &mut fd.body)
}
/// Creates a concrete syntax tree from the lexer.
pub fn make_cst(l: &mut Lexer, tree: &mut ConcreteFileTree) -> i32 {
    tree.decls = create_list(16);

    let mut fd = FunctionDeclaration {
        body: BlockStatement {
            stmts: create_list(16),
        },
        name: String::new(),
    };

    if parse_funcdecl(l, &mut fd) != 0 {
        return -1;
    }

    let decl = TopLevelDeclaration {
        fd,
        node_type: NodeType::NT_FUNCDECL,
    };
    ladd_element(&mut tree.decls, Box::new(decl))
}
/// Parses an expression from the Lexer into an Expression object.
pub fn parse_expr(l: &mut Lexer, ex: &mut Expression) -> i32 {
    let mut token = empty_token();
    if lex(l, &mut token) != 0 {
        return -1;
    }

    match token.token_type {
        TokenType::TT_LITERAL => {
            ex.literal = Some(token.contents);
            ex.fc = None;
            ex.node_type = NodeType::NT_LITERAL;
            0
        }
        TokenType::TT_IDENTIFIER => {
            let name = std::mem::take(&mut token.contents);
            if lex(l, &mut token) != 0 {
                return -1;
            }
            if token.token_type != TokenType::TT_OPAREN {
                return -1;
            }
            if lex(l, &mut token) != 0 || token.token_type != TokenType::TT_CPAREN {
                return -1;
            }
            ex.fc = Some(FunctionCall { name });
            ex.literal = None;
            ex.node_type = NodeType::NT_FUNCCALL;
            0
        }
        _ => -1,
    }
}
/// Parses a file and returns a status code.
pub fn parse(filename: &str) -> i32 {
    let Some(mut lexer) = make_lexer(filename) else {
        eprintln!("File {} not found", filename);
        return 1;
    };

    let mut tokens: Vec<Token> = Vec::new();
    loop {
        let mut t = empty_token();
        if lex(&mut lexer, &mut t) != 0 {
            return 1;
        }

        println!(
            "Contents: {:>20}, type: {:>20}, position: {}/{}",
            t.contents,
            crate::lex::token_type_name(&t.token_type),
            t.line,
            t.column
        );

        let is_eof = t.token_type == TokenType::TT_EOF;
        tokens.push(t);
        if is_eof {
            break;
        }
    }

    if tokens.len() >= 9
        && tokens[0].token_type == TokenType::TT_INT
        && tokens[1].token_type == TokenType::TT_IDENTIFIER
        && tokens[1].contents == "main"
    {
        if tokens[2].token_type == TokenType::TT_OPAREN
            && tokens[3].token_type == TokenType::TT_CPAREN
            && tokens[4].token_type == TokenType::TT_OBRACE
        {
            if tokens[5].token_type == TokenType::TT_RETURN
                && tokens[6].token_type == TokenType::TT_LITERAL
                && tokens[6]
                    .contents
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_digit())
                && tokens[7].token_type == TokenType::TT_SEMI
            {
                if tokens[8].token_type == TokenType::TT_CBRACE {
                    println!();
                    print!("{}", crate::codegen::start_main());
                    let value = tokens[6].contents.parse::<i32>().unwrap_or(0);
                    print!("{}", crate::codegen::end_main_custom_return(value));
                } else {
                    eprintln!("Wrong closing brace.");
                }
            } else {
                eprintln!("Return value is wrong.");
            }
        } else {
            eprintln!("Wrong main function body.");
        }
    } else {
        eprintln!("Not correct main function.");
    }

    0
}
/// Parses a simple main function (for testing).
pub fn parse_simple_main_func() -> i32 {
    parse("c_src/tests/simplemain.c")
}
/// Parses a block statement from the lexer.
pub fn parse_blockstmt(l: &mut Lexer, bs: &mut BlockStatement) -> i32 {
    let mut open = empty_token();
    if lex(l, &mut open) != 0 || open.token_type != TokenType::TT_OBRACE {
        return -1;
    }

    let mut keyword = empty_token();
    if lex(l, &mut keyword) != 0 || keyword.token_type != TokenType::TT_RETURN {
        return -1;
    }

    let mut expr = Expression {
        fc: None,
        literal: None,
        node_type: NodeType::NT_EXPR,
    };
    if parse_expr(l, &mut expr) != 0 {
        return -1;
    }

    let mut semi = empty_token();
    let mut close = empty_token();
    if lex(l, &mut semi) != 0 || semi.token_type != TokenType::TT_SEMI {
        return -1;
    }
    if lex(l, &mut close) != 0 || close.token_type != TokenType::TT_CBRACE {
        return -1;
    }

    bs.stmts = create_list(16);
    ladd_element(&mut bs.stmts, Box::new(expr))
}
/// Parses a function call from the lexer into an Expression object.
pub fn parse_funccall(l: &mut Lexer, ex: &mut Expression) -> i32 {
    let mut name = empty_token();
    let mut open = empty_token();
    let mut close = empty_token();

    if lex(l, &mut name) != 0 || name.token_type != TokenType::TT_IDENTIFIER {
        return -1;
    }
    if lex(l, &mut open) != 0 || open.token_type != TokenType::TT_OPAREN {
        return -1;
    }
    if lex(l, &mut close) != 0 || close.token_type != TokenType::TT_CPAREN {
        return -1;
    }

    ex.fc = Some(FunctionCall {
        name: name.contents,
    });
    ex.literal = None;
    ex.node_type = NodeType::NT_FUNCCALL;
    0
}
