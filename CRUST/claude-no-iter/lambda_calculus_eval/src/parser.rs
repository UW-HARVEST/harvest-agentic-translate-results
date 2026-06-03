use crate::{common, hash_table, io};
use crate::common::{AstNode, AstNodeType, AstNodeUnion, LambdaExpression, Application, Variable, tokens_t};
use std::fs::File;
use std::io::{Seek, SeekFrom};
use std::sync::atomic::{AtomicI32, Ordering};

static N_COUNTER: AtomicI32 = AtomicI32::new(1);

pub fn parse_token(token: char) -> common::tokens_t {
    if token == '(' {
        tokens_t::L_PAREN
    } else if token == ')' {
        tokens_t::R_PAREN
    } else if token == '@' {
        tokens_t::LAMBDA
    } else if token == '.' {
        tokens_t::DOT
    } else if is_variable(token) {
        tokens_t::VARIABLE
    } else if token == ' ' {
        tokens_t::WHITESPACE
    } else if token == '\n' {
        tokens_t::NEWLINE
    } else if token == '=' {
        tokens_t::EQ
    } else if token == '"' {
        tokens_t::QUOTE
    } else if token == ':' {
        tokens_t::COLON
    } else {
        tokens_t::ERROR
    }
}
pub fn p_print_token(token: common::tokens_t) {
    match token {
        tokens_t::L_PAREN => print!("( "),
        tokens_t::R_PAREN => print!(") "),
        tokens_t::LAMBDA => print!("@ "),
        tokens_t::DOT => print!(". "),
        tokens_t::VARIABLE => print!("VARIABLE "),
        tokens_t::WHITESPACE => print!("WHITESPACE "),
        tokens_t::NEWLINE => print!("NEWLINE "),
        tokens_t::EQ => print!("= "),
        _ => print!("ERROR "),
    }
}
pub fn p_print_astNode_type(n: &common::AstNode) {
    match n.type_ {
        AstNodeType::LAMBDA_EXPR => println!("AstNode Type: LAMBDA_EXPR"),
        AstNodeType::APPLICATION => println!("AstNode Type: APPLICATION"),
        AstNodeType::VAR => println!("AstNode Type: VAR"),
        AstNodeType::DEFINITION => println!("AstNode Type: DEFINITION"),
    }
}
pub fn print_ast(node: &common::AstNode) {
    match node.type_ {
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(ref lambda) = node.node {
                print!("(LAMBDA {} : {}", lambda.parameter, lambda.type_);
                if let Some(ref body) = lambda.body {
                    print_ast(body);
                }
                print!(") ");
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(ref app) = node.node {
                print!("(APP ");
                if let Some(ref f) = app.function {
                    print_ast(f);
                }
                if let Some(ref a) = app.argument {
                    print_ast(a);
                }
                print!(") ");
            }
        }
        AstNodeType::VAR => {
            if let AstNodeUnion::Variable(ref v) = node.node {
                print!("(VAR {} ", v.name);
                if !v.type_.is_empty() {
                    print!(": {}", v.type_);
                }
                print!(")");
            }
        }
        AstNodeType::DEFINITION => {
            if let AstNodeUnion::Variable(ref v) = node.node {
                print!("(DEFINITION {}) ", v.name);
            }
        }
    }
}
pub fn is_variable(token: char) -> bool {
    let cmp = token as u32;
    if cmp == '_' as u32 {
        return true;
    }
    (cmp >= 97 && cmp <= 122) || (cmp >= 65 && cmp <= 90)
}
pub fn peek(in_: &mut File) -> char {
    let cur = in_.stream_position().unwrap_or(0);
    let c = match io::next(in_) {
        Ok(c) => c,
        Err(_) => '\u{FFFF}',
    };
    let _ = in_.seek(SeekFrom::Start(cur));
    c
}
pub fn peek_print(in_: &mut File, n: usize) {
    let cur = in_.stream_position().unwrap_or(0);
    let mut buf = String::new();
    for _ in 0..n {
        match io::next(in_) {
            Ok(c) if c != '\u{FFFF}' => buf.push(c),
            _ => break,
        }
    }
    print!("{}", buf);
    let _ = in_.seek(SeekFrom::Start(cur));
}
pub fn consume(t: common::tokens_t, in_: &mut File, expected: &str) {
    let c = io::next(in_).unwrap_or('\u{FFFF}');
    let p = parse_token(c);
    if p != t {
        expect(expected, c);
    }
}
pub fn create_variable(name: &str, type_: &str) -> common::AstNode {
    AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: name.to_string(),
            type_: type_.to_string(),
        }),
    }
}
pub fn create_application(function: &common::AstNode, argument: &common::AstNode) -> common::AstNode {
    AstNode {
        type_: AstNodeType::APPLICATION,
        node: AstNodeUnion::Application(Application {
            function: Some(Box::new(crate::reducer::deepcopy(function))),
            argument: Some(Box::new(crate::reducer::deepcopy(argument))),
        }),
    }
}
pub fn create_lambda(variable: &str, body: &common::AstNode, type_: &str) -> common::AstNode {
    AstNode {
        type_: AstNodeType::LAMBDA_EXPR,
        node: AstNodeUnion::LambdaExpr(LambdaExpression {
            parameter: variable.to_string(),
            type_: type_.to_string(),
            body: Some(Box::new(crate::reducer::deepcopy(body))),
        }),
    }
}
pub fn alpha_convert(old: &str) -> String {
    let n = N_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("{}_{}", old, n)
}
pub fn is_used(table: &hash_table::HashTable, variable: &str) -> bool {
    table.table_exists(variable)
}
pub fn parse_space_chars(in_: &mut File) {
    let mut c = peek(in_);
    while c == ' ' || c == '\n' || c == '\t' {
        let _ = io::next(in_);
        c = peek(in_);
    }
}
pub fn parse_lambda(table: &mut hash_table::HashTable, in_: &mut File) -> common::AstNode {
    if parse_token(peek(in_)) != tokens_t::VARIABLE {
        expect("A variable", peek(in_));
    }

    let var = parse_variable(in_);
    let mut new_var: Option<String> = None;
    if is_used(table, &var) {
        if table.search(&var).is_some() {
            eprintln!(
                "A definition with name {} already exists. Cannot use same name for lambda abstraction.",
                var
            );
            std::process::exit(1);
        }
        let nv = alpha_convert(&var);
        table.insert(&nv, AstNode::default());
        new_var = Some(nv);
    } else {
        table.insert(&var, AstNode::default());
    }

    parse_space_chars(in_);

    consume(tokens_t::COLON, in_, ":");

    parse_space_chars(in_);

    if parse_token(peek(in_)) != tokens_t::VARIABLE {
        eprintln!("ERROR: Lambda abstractions should be typed.");
        std::process::exit(1);
    }
    let type_ = parse_type(table, in_);

    consume(tokens_t::DOT, in_, ".");

    let mut body = parse_expression(table, in_);

    if let Some(ref nv) = new_var {
        crate::reducer::replace(&mut body, &var, nv);
        return create_lambda(nv, &body, &type_);
    }
    create_lambda(&var, &body, &type_)
}
pub fn parse_expression(table: &mut hash_table::HashTable, in_: &mut File) -> common::AstNode {
    while parse_token(peek(in_)) == tokens_t::WHITESPACE
        || parse_token(peek(in_)) == tokens_t::NEWLINE
    {
        let _ = io::next(in_);
    }
    let scanned = parse_token(peek(in_));

    if scanned == tokens_t::ERROR {
        println!("Error: {} is  a valid token", peek(in_));
        std::process::exit(1);
    }

    if scanned == tokens_t::LAMBDA {
        let _ = io::next(in_);
        return parse_lambda(table, in_);
    } else if scanned == tokens_t::L_PAREN {
        let _ = io::next(in_);
        let expr = parse_expression(table, in_);
        print_ast(&expr);
        let next_token = parse_token(peek(in_));

        if next_token == tokens_t::WHITESPACE {
            let expr_2 = parse_expression(table, in_);
            let application = AstNode {
                type_: AstNodeType::APPLICATION,
                node: AstNodeUnion::Application(Application {
                    function: Some(Box::new(expr)),
                    argument: Some(Box::new(expr_2)),
                }),
            };
            consume(tokens_t::R_PAREN, in_, ")");
            return application;
        }
        consume(tokens_t::R_PAREN, in_, ")");
        return expr;
    } else if scanned == tokens_t::VARIABLE {
        let var_name = parse_variable(in_);

        if var_name == "def" {
            parse_definition(table, in_);
            if peek(in_) != '\u{FFFF}' {
                return parse_expression(table, in_);
            }
            return AstNode::default();
        } else if var_name == "import" {
            parse_import(table, in_);
            if peek(in_) != '\u{FFFF}' {
                return parse_expression(table, in_);
            }
        } else if var_name == "type" {
            parse_type_definition(table, in_);
            if peek(in_) != '\u{FFFF}' {
                return parse_expression(table, in_);
            }
        }

        let mut type_ = String::new();
        parse_space_chars(in_);
        if !table.table_exists(&var_name) {
            if parse_token(peek(in_)) != tokens_t::COLON {
                eprintln!(
                    "Constant Variable {} is not typed. Please provide a type.",
                    var_name
                );
                std::process::exit(1);
            }
            consume(tokens_t::COLON, in_, ":");
            parse_space_chars(in_);
            type_ = parse_type(table, in_);
        }

        let mut variable = create_variable(&var_name, &type_);
        if table.search(&var_name).is_some() {
            variable.type_ = AstNodeType::DEFINITION;
        }
        return variable;
    }
    AstNode::default()
}
pub fn parse_import(table: &mut hash_table::HashTable, in_: &mut File) {
    consume(tokens_t::WHITESPACE, in_, "a whitespace");
    consume(tokens_t::QUOTE, in_, "\"");

    let mut file_path = String::new();
    let mut next_token = io::next(in_).unwrap_or('\u{FFFF}');
    let mut n = parse_token(next_token);

    while n != tokens_t::QUOTE {
        if file_path.len() < 99 {
            file_path.push(next_token);
        } else {
            eprintln!("File path is too long. Please make sure it is less than 100 characters.");
            std::process::exit(1);
        }
        next_token = io::next(in_).unwrap_or('\u{FFFF}');
        n = parse_token(next_token);
    }

    let mut imported_file = match crate::io::get_file(&file_path, "r") {
        Ok(f) => f,
        Err(_) => {
            eprintln!("ERROR: Could not open file {}", file_path);
            std::process::exit(1);
        }
    };

    loop {
        let imported_tkn = peek(&mut imported_file);
        if imported_tkn == '\u{FFFF}' {
            break;
        }
        let scanned = parse_token(imported_tkn);
        parse_space_chars(&mut imported_file);
        if scanned == tokens_t::VARIABLE {
            let var_name = parse_variable(&mut imported_file);
            if var_name == "def" {
                parse_definition(table, &mut imported_file);
            } else if var_name == "type" {
                parse_type_definition(table, &mut imported_file);
            } else {
                eprintln!(
                    "Expected a definition in the imported file, but got {}",
                    var_name
                );
                std::process::exit(1);
            }
        }
    }
}
pub fn parse_definition(table: &mut hash_table::HashTable, in_: &mut File) {
    consume(tokens_t::WHITESPACE, in_, "a whitespace");

    if parse_token(peek(in_)) != tokens_t::VARIABLE {
        expect("a variable", peek(in_));
    }

    let def_name = parse_variable(in_);

    consume(tokens_t::WHITESPACE, in_, "a whitespace");
    consume(tokens_t::EQ, in_, "=");
    consume(tokens_t::WHITESPACE, in_, "a whitespace");

    let definition = parse_expression(table, in_);
    table.insert(&def_name, definition);
}
pub fn is_uppercase(c: char) -> bool {
    c >= 'A' && c <= 'Z'
}
pub fn parse_type_definition(types_table: &mut hash_table::HashTable, in_: &mut File) {
    let next_token = io::next(in_).unwrap_or('\u{FFFF}');
    let n = parse_token(next_token);
    if n != tokens_t::WHITESPACE {
        expect(" ", next_token);
    }

    let next_token = peek(in_);
    let n = parse_token(next_token);
    if n != tokens_t::VARIABLE {
        expect("a type definition", next_token);
    }

    if !is_uppercase(next_token) {
        eprintln!("ERROR: Type names must start with an uppercase letter");
        std::process::exit(1);
    }

    let type_name = parse_variable(in_);
    if types_table.table_exists(&type_name) {
        eprintln!("Type {} was already defined.", type_name);
        std::process::exit(1);
    }
    types_table.insert(&type_name, AstNode::default());
}
pub fn parse_type(types_table: &mut hash_table::HashTable, in_: &mut File) -> String {
    let mut type_name = String::new();
    let token = io::next(in_).unwrap_or('\u{FFFF}');

    if !is_uppercase(token) {
        eprintln!("ERROR: Types should start with an uppercase letter.");
        std::process::exit(1);
    }

    type_name.push(token);

    while is_variable(peek(in_)) {
        let c = io::next(in_).unwrap_or('\u{FFFF}');
        type_name.push(c);
    }

    if !types_table.table_exists(&type_name) {
        eprintln!("Type {} was not defined.", type_name);
        std::process::exit(1);
    }
    type_name
}
pub fn parse_variable(in_: &mut File) -> String {
    let mut variable_name = String::new();
    while is_variable(peek(in_)) {
        let c = io::next(in_).unwrap_or('\u{FFFF}');
        variable_name.push(c);
    }
    variable_name
}
pub fn expect(expected: &str, received: char) {
    println!("Syntax Error: Expected {} , received {} ", expected, received);
    std::process::exit(1);
}
pub fn free_ast(_node: &mut common::AstNode) {
    // In Rust, dropping the AstNode will free everything. No-op.
}
