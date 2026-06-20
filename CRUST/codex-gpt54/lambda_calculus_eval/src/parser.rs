use crate::{common, hash_table, io, reducer};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicUsize, Ordering};

static ALPHA_COUNTER: AtomicUsize = AtomicUsize::new(1);

fn fatal(msg: &str, file: &str, line: i32, func: &str) -> ! {
    common::error(msg, file, line, func);
    std::process::exit(1);
}

fn read_char(in_: &mut File) -> char {
    io::next(in_).unwrap_or('\0')
}

fn peek_char(in_: &mut File) -> char {
    let position = in_.stream_position().unwrap_or(0);
    let c = read_char(in_);
    let _ = in_.seek(SeekFrom::Start(position));
    c
}

pub fn parse_token(token: char) -> common::tokens_t {
    match token {
        '(' => common::tokens_t::L_PAREN,
        ')' => common::tokens_t::R_PAREN,
        '@' => common::tokens_t::LAMBDA,
        '.' => common::tokens_t::DOT,
        ' ' => common::tokens_t::WHITESPACE,
        '\n' => common::tokens_t::NEWLINE,
        '=' => common::tokens_t::EQ,
        '"' => common::tokens_t::QUOTE,
        ':' => common::tokens_t::COLON,
        _ if is_variable(token) => common::tokens_t::VARIABLE,
        _ => common::tokens_t::ERROR,
    }
}

pub fn p_print_token(token: common::tokens_t) {
    match token {
        common::tokens_t::L_PAREN => print!("( "),
        common::tokens_t::R_PAREN => print!(") "),
        common::tokens_t::LAMBDA => print!("@ "),
        common::tokens_t::DOT => print!(". "),
        common::tokens_t::VARIABLE => print!("VARIABLE "),
        common::tokens_t::WHITESPACE => print!("WHITESPACE "),
        common::tokens_t::NEWLINE => print!("NEWLINE "),
        common::tokens_t::EQ => print!("= "),
        _ => print!("ERROR "),
    }
}

pub fn p_print_astNode_type(n: &common::AstNode) {
    match n.type_ {
        common::AstNodeType::LAMBDA_EXPR => println!("AstNode Type: LAMBDA_EXPR"),
        common::AstNodeType::APPLICATION => println!("AstNode Type: APPLICATION"),
        common::AstNodeType::VAR => println!("AstNode Type: VAR"),
        common::AstNodeType::DEFINITION => println!("AstNode Type: DEFINITION"),
    }
}

pub fn print_ast(node: &common::AstNode) {
    match (&node.type_, &node.node) {
        (common::AstNodeType::LAMBDA_EXPR, common::AstNodeUnion::LambdaExpr(lambda)) => {
            print!("(LAMBDA {} : {}", lambda.parameter, lambda.type_);
            if let Some(body) = &lambda.body {
                print_ast(body);
            }
            print!(") ");
        }
        (common::AstNodeType::APPLICATION, common::AstNodeUnion::Application(application)) => {
            print!("(APP ");
            if let Some(function) = &application.function {
                print_ast(function);
            }
            if let Some(argument) = &application.argument {
                print_ast(argument);
            }
            print!(") ");
        }
        (common::AstNodeType::VAR, common::AstNodeUnion::Variable(variable)) => {
            print!("(VAR {} ", variable.name);
            if !variable.type_.is_empty() {
                print!(": {}", variable.type_);
            }
            print!(")");
        }
        (common::AstNodeType::DEFINITION, common::AstNodeUnion::Variable(variable)) => {
            print!("(DEFINITION {}) ", variable.name);
        }
        _ => print!("(UNKNOWN) "),
    }
}

pub fn is_variable(token: char) -> bool {
    token == '_' || token.is_ascii_alphabetic()
}

pub fn peek(in_: &mut File) -> char {
    peek_char(in_)
}

pub fn peek_print(in_: &mut File, n: usize) {
    let position = in_.stream_position().unwrap_or(0);
    let mut buffer = vec![0_u8; n];
    let count = in_.read(&mut buffer).unwrap_or(0);
    print!("{}", String::from_utf8_lossy(&buffer[..count]));
    let _ = in_.seek(SeekFrom::Start(position));
}

pub fn consume(t: common::tokens_t, in_: &mut File, expected: &str) {
    let c = read_char(in_);
    let p = parse_token(c);
    if p != t {
        expect(expected, c);
    }
}

pub fn create_variable(name: &str, type_: &str) -> common::AstNode {
    common::AstNode {
        type_: common::AstNodeType::VAR,
        node: common::AstNodeUnion::Variable(common::Variable {
            name: name.to_string(),
            type_: type_.to_string(),
        }),
    }
}

pub fn create_application(
    function: &common::AstNode,
    argument: &common::AstNode,
) -> common::AstNode {
    common::AstNode {
        type_: common::AstNodeType::APPLICATION,
        node: common::AstNodeUnion::Application(common::Application {
            function: Some(Box::new(function.clone())),
            argument: Some(Box::new(argument.clone())),
        }),
    }
}

pub fn create_lambda(variable: &str, body: &common::AstNode, type_: &str) -> common::AstNode {
    common::AstNode {
        type_: common::AstNodeType::LAMBDA_EXPR,
        node: common::AstNodeUnion::LambdaExpr(common::LambdaExpression {
            parameter: variable.to_string(),
            type_: type_.to_string(),
            body: Some(Box::new(body.clone())),
        }),
    }
}

pub fn alpha_convert(old: &str) -> String {
    let n = ALPHA_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{old}_{n}")
}

pub fn is_used(table: &hash_table::HashTable, variable: &str) -> bool {
    table.table_exists(variable)
}

pub fn parse_space_chars(in_: &mut File) {
    let mut c = peek(in_);
    while matches!(c, ' ' | '\n' | '\t') {
        let _ = read_char(in_);
        c = peek(in_);
    }
}

pub fn parse_lambda(table: &mut hash_table::HashTable, in_: &mut File) -> common::AstNode {
    if parse_token(peek(in_)) != common::tokens_t::VARIABLE {
        expect("A variable", '\0');
    }

    let var = parse_variable(in_);
    let mut chosen_var = var.clone();
    if is_used(table, &var) {
        if table.search(&var).is_some() {
            fatal(
                &format!(
                    "A definition with name {var} already exists. Cannot use same name for lambda abstraction.\n"
                ),
                file!(),
                line!() as i32,
                "parse_lambda",
            );
        }
        chosen_var = alpha_convert(&var);
        table.insert_placeholder(&chosen_var);
    } else {
        table.insert_placeholder(&var);
    }

    parse_space_chars(in_);
    consume(common::tokens_t::COLON, in_, ":");
    parse_space_chars(in_);

    if parse_token(peek(in_)) != common::tokens_t::VARIABLE {
        fatal(
            "Lambda abstractions should be typed.",
            file!(),
            line!() as i32,
            "parse_lambda",
        );
    }
    let type_ = parse_type(table, in_);
    consume(common::tokens_t::DOT, in_, ".");

    let mut body = parse_expression(table, in_);
    if chosen_var != var {
        reducer::replace(&mut body, &var, &chosen_var);
        common::print_verbose("", format_args!("Alpha converted {var} to {chosen_var}\n"));
    }
    create_lambda(&chosen_var, &body, &type_)
}

pub fn parse_expression(table: &mut hash_table::HashTable, in_: &mut File) -> common::AstNode {
    while matches!(
        parse_token(peek(in_)),
        common::tokens_t::WHITESPACE | common::tokens_t::NEWLINE
    ) {
        let _ = read_char(in_);
    }

    let scanned = parse_token(peek(in_));
    if scanned == common::tokens_t::ERROR {
        let current = peek(in_);
        if current == '\0' {
            return common::AstNode::default();
        }
        eprintln!("Error: {current} is  a valid token");
        std::process::exit(1);
    }

    if scanned == common::tokens_t::LAMBDA {
        let _ = read_char(in_);
        return parse_lambda(table, in_);
    } else if scanned == common::tokens_t::L_PAREN {
        let _ = read_char(in_);
        let expr = parse_expression(table, in_);

        print_ast(&expr);
        let next_token = parse_token(peek(in_));
        if next_token == common::tokens_t::WHITESPACE {
            let expr_2 = parse_expression(table, in_);
            let application = create_application(&expr, &expr_2);
            consume(common::tokens_t::R_PAREN, in_, ")");
            return application;
        }
        consume(common::tokens_t::R_PAREN, in_, ")");
        return expr;
    } else if scanned == common::tokens_t::VARIABLE {
        let var_name = parse_variable(in_);

        if var_name == "def" {
            parse_definition(table, in_);
            if peek(in_) != '\0' {
                return parse_expression(table, in_);
            }
            return common::AstNode::default();
        } else if var_name == "import" {
            parse_import(table, in_);
            if peek(in_) != '\0' {
                return parse_expression(table, in_);
            }
            return common::AstNode::default();
        } else if var_name == "type" {
            parse_type_definition(table, in_);
            if peek(in_) != '\0' {
                return parse_expression(table, in_);
            }
            return common::AstNode::default();
        }

        let mut type_ = String::new();
        parse_space_chars(in_);
        if !table.table_exists(&var_name) {
            if parse_token(peek(in_)) != common::tokens_t::COLON {
                fatal(
                    &format!(
                        "Constant Variable {var_name} is not typed. Please provide a type.\n"
                    ),
                    file!(),
                    line!() as i32,
                    "parse_expression",
                );
            }
            consume(common::tokens_t::COLON, in_, ":");
            parse_space_chars(in_);
            type_ = parse_type(table, in_);
        }

        let mut variable = create_variable(&var_name, &type_);
        if table.search(&var_name).is_some() {
            variable.type_ = common::AstNodeType::DEFINITION;
        }
        return variable;
    }

    common::AstNode::default()
}

pub fn parse_import(table: &mut hash_table::HashTable, in_: &mut File) {
    consume(common::tokens_t::WHITESPACE, in_, "a whitespace");
    consume(common::tokens_t::QUOTE, in_, "\"");

    let mut file_path = String::new();
    let mut next_token = read_char(in_);
    let mut token_type = parse_token(next_token);

    while token_type != common::tokens_t::QUOTE {
        if file_path.len() >= 99 {
            fatal(
                "File path is too long. Please make sure it is less than 100 characters.",
                file!(),
                line!() as i32,
                "parse_import",
            );
        }
        file_path.push(next_token);
        next_token = read_char(in_);
        token_type = parse_token(next_token);
    }

    let mut imported_file = io::get_file(&file_path, "r").unwrap_or_else(|_| {
        fatal(
            &format!("ERROR: Could not open file {file_path}\n"),
            file!(),
            line!() as i32,
            "parse_import",
        )
    });

    while peek(&mut imported_file) != '\0' {
        parse_space_chars(&mut imported_file);
        let imported_tkn = peek(&mut imported_file);
        if imported_tkn == '\0' {
            break;
        }
        if parse_token(imported_tkn) == common::tokens_t::VARIABLE {
            let var_name = parse_variable(&mut imported_file);
            if var_name == "def" {
                parse_definition(table, &mut imported_file);
            } else if var_name == "type" {
                parse_type_definition(table, &mut imported_file);
            } else {
                fatal(
                    &format!(
                        "Expected a definition in the imported file, but got {var_name}\n"
                    ),
                    file!(),
                    line!() as i32,
                    "parse_import",
                );
            }
        }
    }
}

pub fn parse_definition(table: &mut hash_table::HashTable, in_: &mut File) {
    consume(common::tokens_t::WHITESPACE, in_, "a whitespace");
    if parse_token(peek(in_)) != common::tokens_t::VARIABLE {
        expect("a variable", '\0');
    }

    let def_name = parse_variable(in_);
    consume(common::tokens_t::WHITESPACE, in_, "a whitespace");
    consume(common::tokens_t::EQ, in_, "=");
    consume(common::tokens_t::WHITESPACE, in_, "a whitespace");

    let definition = parse_expression(table, in_);
    table.insert(&def_name, definition);
}

pub fn is_uppercase(c: char) -> bool {
    c.is_ascii_uppercase()
}

pub fn parse_type_definition(types_table: &mut hash_table::HashTable, in_: &mut File) {
    let next_token = read_char(in_);
    if parse_token(next_token) != common::tokens_t::WHITESPACE {
        expect(" ", next_token);
    }

    let next_token = peek(in_);
    if parse_token(next_token) != common::tokens_t::VARIABLE {
        expect("a type definition", next_token);
    }
    if !is_uppercase(next_token) {
        fatal(
            "Type names must start with an uppercase letter",
            file!(),
            line!() as i32,
            "parse_type_definition",
        );
    }

    let type_name = parse_variable(in_);
    if types_table.table_exists(&type_name) {
        fatal(
            &format!("Type {type_name} was already defined.\n"),
            file!(),
            line!() as i32,
            "parse_type_definition",
        );
    }
    types_table.insert_placeholder(&type_name);
}

pub fn parse_type(types_table: &mut hash_table::HashTable, in_: &mut File) -> String {
    let mut type_name = String::new();
    let token = read_char(in_);
    if !is_uppercase(token) {
        fatal(
            "Types should start with an uppercase letter.",
            file!(),
            line!() as i32,
            "parse_type",
        );
    }

    type_name.push(token);
    while is_variable(peek(in_)) {
        type_name.push(read_char(in_));
    }

    if !types_table.table_exists(&type_name) {
        fatal(
            &format!("Type {type_name} was not defined.\n"),
            file!(),
            line!() as i32,
            "parse_type",
        );
    }
    type_name
}

pub fn parse_variable(in_: &mut File) -> String {
    let mut variable_name = String::new();
    while is_variable(peek(in_)) {
        variable_name.push(read_char(in_));
    }
    variable_name
}

pub fn expect(expected: &str, received: char) {
    println!("Syntax Error: Expected {expected} , received {received} ");
    std::process::exit(1);
}

pub fn free_ast(_node: &mut common::AstNode) {}
