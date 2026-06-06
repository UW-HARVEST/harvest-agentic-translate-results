use crate::common::{self, AstNode, AstNodeType, AstNodeUnion, Application, LambdaExpression, Variable};
use crate::hash_table::{self, HashTable};
use crate::io as my_io;
use crate::reducer;
use std::cell::Cell;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

thread_local! {
    static N: Cell<i32> = Cell::new(1);
}

pub fn parse_token(token: char) -> common::tokens_t {
    if token == '(' {
        common::tokens_t::L_PAREN
    } else if token == ')' {
        common::tokens_t::R_PAREN
    } else if token == '@' {
        common::tokens_t::LAMBDA
    } else if token == '.' {
        common::tokens_t::DOT
    } else if is_variable(token) {
        common::tokens_t::VARIABLE
    } else if token == ' ' {
        common::tokens_t::WHITESPACE
    } else if token == '\n' {
        common::tokens_t::NEWLINE
    } else if token == '=' {
        common::tokens_t::EQ
    } else if token == '"' {
        common::tokens_t::QUOTE
    } else if token == ':' {
        common::tokens_t::COLON
    } else {
        common::tokens_t::ERROR
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

pub fn p_print_astNode_type(n: &AstNode) {
    match n.type_ {
        AstNodeType::LAMBDA_EXPR => println!("AstNode Type: LAMBDA_EXPR"),
        AstNodeType::APPLICATION => println!("AstNode Type: APPLICATION"),
        AstNodeType::VAR => println!("AstNode Type: VAR"),
        AstNodeType::DEFINITION => println!("AstNode Type: DEFINITION"),
    }
}

pub fn print_ast(node: &AstNode) {
    match &node.node {
        AstNodeUnion::LambdaExpr(le) => {
            print!("(LAMBDA {} : {}", le.parameter, le.type_);
            if let Some(body) = &le.body {
                print_ast(body);
            }
            print!(") ");
        }
        AstNodeUnion::Application(app) => {
            print!("(APP ");
            if let Some(f) = &app.function {
                print_ast(f);
            }
            if let Some(a) = &app.argument {
                print_ast(a);
            }
            print!(") ");
        }
        AstNodeUnion::Variable(v) => {
            match node.type_ {
                AstNodeType::DEFINITION => {
                    print!("(DEFINITION {}) ", v.name);
                }
                _ => {
                    print!("(VAR {} ", v.name);
                    if !v.type_.is_empty() {
                        print!(": {}", v.type_);
                    }
                    print!(")");
                }
            }
        }
    }
}

pub fn is_variable(token: char) -> bool {
    let cmp = token as i32;
    if cmp == '_' as i32 {
        return true;
    }
    if (cmp >= 97 && cmp <= 122) || (cmp >= 65 && cmp <= 90) {
        return true;
    }
    false
}

pub fn peek(in_: &mut File) -> char {
    let mut buf = [0u8; 1];
    let n = match in_.read(&mut buf) {
        Ok(n) => n,
        Err(_) => 0,
    };
    if n == 0 {
        return '\u{FFFF}'; // EOF marker
    }
    // unread by seeking back
    let _ = in_.seek(SeekFrom::Current(-1));
    buf[0] as char
}

pub fn peek_print(in_: &mut File, n: usize) {
    let mut buffer = Vec::with_capacity(n);
    for _ in 0..n {
        let mut b = [0u8; 1];
        match in_.read(&mut b) {
            Ok(0) => break,
            Ok(_) => buffer.push(b[0]),
            Err(_) => break,
        }
    }
    let s: String = buffer.iter().map(|&b| b as char).collect();
    print!("{}", s);
    let len = buffer.len() as i64;
    let _ = in_.seek(SeekFrom::Current(-len));
}

pub fn consume(t: common::tokens_t, in_: &mut File, expected: &str) {
    let c = match my_io::next(in_) {
        Ok(c) => c,
        Err(_) => '\u{FFFF}',
    };
    let p = parse_token(c);
    if p != t {
        expect(expected, c);
    }
}

pub fn create_variable(name: &str, type_: &str) -> AstNode {
    AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: name.to_string(),
            type_: type_.to_string(),
        }),
    }
}

pub fn create_application(function: &AstNode, argument: &AstNode) -> AstNode {
    AstNode {
        type_: AstNodeType::APPLICATION,
        node: AstNodeUnion::Application(Application {
            function: Some(Box::new(function.clone())),
            argument: Some(Box::new(argument.clone())),
        }),
    }
}

pub fn create_lambda(variable: &str, body: &AstNode, type_: &str) -> AstNode {
    AstNode {
        type_: AstNodeType::LAMBDA_EXPR,
        node: AstNodeUnion::LambdaExpr(LambdaExpression {
            parameter: variable.to_string(),
            type_: type_.to_string(),
            body: Some(Box::new(body.clone())),
        }),
    }
}

pub fn alpha_convert(old: &str) -> String {
    let n = N.with(|c| {
        let cur = c.get();
        c.set(cur + 1);
        cur
    });
    format!("{}_{}", old, n)
}

pub fn is_used(table: &HashTable, variable: &str) -> bool {
    table.table_exists(variable)
}

pub fn parse_space_chars(in_: &mut File) {
    loop {
        let c = peek(in_);
        if c == ' ' || c == '\n' || c == '\t' {
            let _ = my_io::next(in_);
        } else {
            break;
        }
    }
}

pub fn parse_lambda(table: &mut HashTable, in_: &mut File) -> AstNode {
    if parse_token(peek(in_)) != common::tokens_t::VARIABLE {
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

    consume(common::tokens_t::COLON, in_, ":");

    parse_space_chars(in_);

    if parse_token(peek(in_)) != common::tokens_t::VARIABLE {
        eprintln!("Lambda abstractions should be typed.");
        std::process::exit(1);
    }
    let type_ = parse_type(table, in_);

    consume(common::tokens_t::DOT, in_, ".");

    let mut body = parse_expression(table, in_);

    if let Some(nv) = new_var {
        reducer::replace(&mut body, &var, &nv);
        create_lambda(&nv, &body, &type_)
    } else {
        create_lambda(&var, &body, &type_)
    }
}

pub fn parse_expression(table: &mut HashTable, in_: &mut File) -> AstNode {
    while parse_token(peek(in_)) == common::tokens_t::WHITESPACE
        || parse_token(peek(in_)) == common::tokens_t::NEWLINE
    {
        let _ = my_io::next(in_);
    }
    let scanned = parse_token(peek(in_));

    if scanned == common::tokens_t::ERROR {
        println!("Error: {} is  a valid token", peek(in_));
        std::process::exit(1);
    }

    if scanned == common::tokens_t::LAMBDA {
        let _ = my_io::next(in_);
        return parse_lambda(table, in_);
    } else if scanned == common::tokens_t::L_PAREN {
        let _ = my_io::next(in_);
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
            if parse_token(peek(in_)) != common::tokens_t::COLON {
                eprintln!("Constant Variable {} is not typed. Please provide a type.", var_name);
                std::process::exit(1);
            }
            consume(common::tokens_t::COLON, in_, ":");
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

pub fn parse_import(table: &mut HashTable, in_: &mut File) {
    consume(common::tokens_t::WHITESPACE, in_, "a whitespace");
    consume(common::tokens_t::QUOTE, in_, "\"");

    let mut file_path = String::new();
    let mut next_token = match my_io::next(in_) {
        Ok(c) => c,
        Err(_) => '\u{FFFF}',
    };
    let mut n = parse_token(next_token);

    while n != common::tokens_t::QUOTE {
        if file_path.len() < 99 {
            file_path.push(next_token);
        } else {
            eprintln!("File path is too long. Please make sure it is less than 100 characters.");
            std::process::exit(1);
        }
        next_token = match my_io::next(in_) {
            Ok(c) => c,
            Err(_) => '\u{FFFF}',
        };
        n = parse_token(next_token);
    }

    let mut imported_file =
        my_io::get_file(&file_path, "r").expect("Could not open imported file");

    loop {
        let imported_tkn = peek(&mut imported_file);
        if imported_tkn == '\u{FFFF}' {
            break;
        }
        let scanned = parse_token(imported_tkn);
        parse_space_chars(&mut imported_file);
        if scanned == common::tokens_t::VARIABLE {
            let var_name = parse_variable(&mut imported_file);
            if var_name == "def" {
                parse_definition(table, &mut imported_file);
            } else if var_name == "type" {
                parse_type_definition(table, &mut imported_file);
            } else {
                eprintln!("Expected a definition in the imported file, but got {}", var_name);
                std::process::exit(1);
            }
        }
    }
}

pub fn parse_definition(table: &mut HashTable, in_: &mut File) {
    consume(common::tokens_t::WHITESPACE, in_, "a whitespace");

    if parse_token(peek(in_)) != common::tokens_t::VARIABLE {
        expect("a variable", peek(in_));
    }

    let def_name = parse_variable(in_);

    consume(common::tokens_t::WHITESPACE, in_, "a whitespace");
    consume(common::tokens_t::EQ, in_, "=");
    consume(common::tokens_t::WHITESPACE, in_, "a whitespace");

    let definition = parse_expression(table, in_);
    table.insert(&def_name, definition);
}

pub fn is_uppercase(c: char) -> bool {
    c >= 'A' && c <= 'Z'
}

pub fn parse_type_definition(types_table: &mut HashTable, in_: &mut File) {
    let next_token = match my_io::next(in_) {
        Ok(c) => c,
        Err(_) => '\u{FFFF}',
    };
    let n = parse_token(next_token);
    if n != common::tokens_t::WHITESPACE {
        expect(" ", next_token);
    }

    let next_token2 = peek(in_);
    let n2 = parse_token(next_token2);
    if n2 != common::tokens_t::VARIABLE {
        expect("a type definition", next_token2);
    }

    if !is_uppercase(next_token2) {
        eprintln!("Type names must start with an uppercase letter");
        std::process::exit(1);
    }

    let type_name = parse_variable(in_);
    if types_table.table_exists(&type_name) {
        eprintln!("Type {} was already defined.", type_name);
        std::process::exit(1);
    }
    types_table.insert(&type_name, AstNode::default());
}

pub fn parse_type(types_table: &mut HashTable, in_: &mut File) -> String {
    let mut type_name = String::new();
    let token = match my_io::next(in_) {
        Ok(c) => c,
        Err(_) => '\u{FFFF}',
    };

    if !is_uppercase(token) {
        eprintln!("Types should start with an uppercase letter.");
        std::process::exit(1);
    }
    type_name.push(token);

    while is_variable(peek(in_)) {
        let c = match my_io::next(in_) {
            Ok(c) => c,
            Err(_) => break,
        };
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
        let c = match my_io::next(in_) {
            Ok(c) => c,
            Err(_) => break,
        };
        variable_name.push(c);
    }
    variable_name
}

pub fn expect(expected: &str, received: char) {
    println!("Syntax Error: Expected {} , received {} ", expected, received);
    std::process::exit(1);
}

pub fn free_ast(_node: &mut AstNode) {
    // Rust handles freeing automatically
}

// Suppress unused warnings for imports kept to mirror C dependencies
#[allow(dead_code)]
fn _suppress_unused() {
    let _ = hash_table::HASH_TABLE_SIZE;
}
