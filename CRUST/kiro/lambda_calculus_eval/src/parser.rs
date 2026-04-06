use crate::{common, hash_table};
use crate::common::{AstNode, AstNodeType, AstNodeUnion, LambdaExpression, Application, Variable, tokens_t};
use crate::io;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicI32, Ordering};

static N: AtomicI32 = AtomicI32::new(1);

pub fn parse_token(token: char) -> tokens_t {
    match token {
        '(' => tokens_t::L_PAREN,
        ')' => tokens_t::R_PAREN,
        '@' => tokens_t::LAMBDA,
        '.' => tokens_t::DOT,
        ' ' => tokens_t::WHITESPACE,
        '\n' => tokens_t::NEWLINE,
        '=' => tokens_t::EQ,
        '"' => tokens_t::QUOTE,
        ':' => tokens_t::COLON,
        c if is_variable(c) => tokens_t::VARIABLE,
        _ => tokens_t::ERROR,
    }
}
pub fn p_print_token(token: tokens_t) {
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
        AstNodeUnion::Variable(var) => {
            if node.type_ == AstNodeType::DEFINITION {
                print!("(DEFINITION {}) ", var.name);
            } else {
                print!("(VAR {} ", var.name);
                if !var.type_.is_empty() {
                    print!(": {}", var.type_);
                }
                print!(")");
            }
        }
    }
}
pub fn is_variable(token: char) -> bool {
    token == '_' || token.is_ascii_alphabetic()
}
pub fn peek(in_: &mut File) -> char {
    let mut buf = [0u8; 1];
    match in_.read(&mut buf) {
        Ok(0) => '\u{FF}', // EOF
        Ok(_) => {
            in_.seek(SeekFrom::Current(-1)).unwrap();
            buf[0] as char
        }
        Err(_) => '\u{FF}',
    }
}
fn peek_is_eof(in_: &mut File) -> bool {
    let mut buf = [0u8; 1];
    match in_.read(&mut buf) {
        Ok(0) => true,
        Ok(_) => {
            in_.seek(SeekFrom::Current(-1)).unwrap();
            false
        }
        Err(_) => true,
    }
}
pub fn peek_print(in_: &mut File, n: usize) {
    let mut buffer = vec![0u8; n];
    let mut count = 0;
    for i in 0..n {
        let mut b = [0u8; 1];
        match in_.read(&mut b) {
            Ok(0) => break,
            Ok(_) => { buffer[i] = b[0]; count += 1; }
            Err(_) => break,
        }
    }
    print!("{}", String::from_utf8_lossy(&buffer[..count]));
    for i in (0..count).rev() {
        in_.seek(SeekFrom::Current(-1)).unwrap();
    }
}
pub fn consume(t: tokens_t, in_: &mut File, expected: &str) {
    let c = next_char(in_);
    let p = parse_token(c);
    if p != t {
        expect(expected, c);
    }
}
fn next_char(in_: &mut File) -> char {
    let mut buf = [0u8; 1];
    match in_.read(&mut buf) {
        Ok(1) => buf[0] as char,
        _ => '\u{FF}',
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
            function: Some(Box::new(deep_copy(function))),
            argument: Some(Box::new(deep_copy(argument))),
        }),
    }
}
pub fn create_lambda(variable: &str, body: &AstNode, type_: &str) -> AstNode {
    AstNode {
        type_: AstNodeType::LAMBDA_EXPR,
        node: AstNodeUnion::LambdaExpr(LambdaExpression {
            parameter: variable.to_string(),
            type_: type_.to_string(),
            body: Some(Box::new(deep_copy(body))),
        }),
    }
}

fn deep_copy(n: &AstNode) -> AstNode {
    match &n.node {
        AstNodeUnion::Variable(v) => AstNode {
            type_: match n.type_ { AstNodeType::DEFINITION => AstNodeType::DEFINITION, AstNodeType::VAR => AstNodeType::VAR, AstNodeType::LAMBDA_EXPR => AstNodeType::LAMBDA_EXPR, AstNodeType::APPLICATION => AstNodeType::APPLICATION },
            node: AstNodeUnion::Variable(Variable { name: v.name.clone(), type_: v.type_.clone() }),
        },
        AstNodeUnion::LambdaExpr(le) => AstNode {
            type_: AstNodeType::LAMBDA_EXPR,
            node: AstNodeUnion::LambdaExpr(LambdaExpression {
                parameter: le.parameter.clone(),
                type_: le.type_.clone(),
                body: le.body.as_ref().map(|b| Box::new(deep_copy(b))),
            }),
        },
        AstNodeUnion::Application(app) => AstNode {
            type_: AstNodeType::APPLICATION,
            node: AstNodeUnion::Application(Application {
                function: app.function.as_ref().map(|f| Box::new(deep_copy(f))),
                argument: app.argument.as_ref().map(|a| Box::new(deep_copy(a))),
            }),
        },
    }
}

pub fn alpha_convert(old: &str) -> String {
    let n = N.fetch_add(1, Ordering::SeqCst);
    format!("{}_{}", old, n)
}
pub fn is_used(table: &hash_table::HashTable, variable: &str) -> bool {
    table.table_exists(variable)
}
pub fn parse_space_chars(in_: &mut File) {
    let mut c = peek(in_);
    while c == ' ' || c == '\n' || c == '\t' {
        next_char(in_);
        c = peek(in_);
    }
}
pub fn parse_lambda(table: &mut hash_table::HashTable, in_: &mut File) -> AstNode {
    if parse_token(peek(in_)) != tokens_t::VARIABLE {
        expect("A variable", peek(in_));
    }

    let var = parse_variable(in_);
    let mut new_var: Option<String> = None;
    if is_used(table, &var) {
        if table.search(&var).is_some() {
            common::error(
                &format!("A definition with name {} already exists. Cannot use same name for lambda abstraction.", var),
                file!(), line!() as i32, "parse_lambda",
            );
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
        common::error("Lambda abstractions should be typed.", file!(), line!() as i32, "parse_lambda");
    }
    let type_ = parse_type(table, in_);
    consume(tokens_t::DOT, in_, ".");

    let mut body = parse_expression(table, in_);

    if let Some(nv) = new_var {
        crate::reducer::replace(&mut body, &var, &nv);
        common::print_verbose("Alpha converted", format_args!("Alpha converted {} to {}\n", var, nv));
        return create_lambda(&nv, &body, &type_);
    }
    create_lambda(&var, &body, &type_)
}
pub fn parse_expression(table: &mut hash_table::HashTable, in_: &mut File) -> AstNode {
    loop {
        let t = parse_token(peek(in_));
        if t == tokens_t::WHITESPACE || t == tokens_t::NEWLINE {
            next_char(in_);
        } else {
            break;
        }
    }
    let scanned = parse_token(peek(in_));

    if scanned == tokens_t::ERROR {
        println!("Error: {} is  a valid token", peek(in_));
        std::process::exit(1);
    }

    if scanned == tokens_t::LAMBDA {
        next_char(in_);
        return parse_lambda(table, in_);
    } else if scanned == tokens_t::L_PAREN {
        next_char(in_);
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
            if !peek_is_eof(in_) {
                return parse_expression(table, in_);
            }
            return AstNode::default();
        } else if var_name == "import" {
            parse_import(table, in_);
            if !peek_is_eof(in_) {
                return parse_expression(table, in_);
            }
        } else if var_name == "type" {
            parse_type_definition(table, in_);
            if !peek_is_eof(in_) {
                return parse_expression(table, in_);
            }
        }

        let mut type_ = String::new();
        parse_space_chars(in_);

        if !table.table_exists(&var_name) {
            if parse_token(peek(in_)) != tokens_t::COLON {
                common::error(
                    &format!("Constant Variable {} is not typed. Please provide a type.", var_name),
                    file!(), line!() as i32, "parse_expression",
                );
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
    let mut next_token = next_char(in_);
    let mut n = parse_token(next_token);

    while n != tokens_t::QUOTE {
        if file_path.len() < 99 {
            file_path.push(next_token);
        } else {
            common::error("File path is too long. Please make sure it is less than 100 characters.",
                file!(), line!() as i32, "parse_import");
        }
        next_token = next_char(in_);
        n = parse_token(next_token);
    }

    let mut imported_file = io::get_file(&file_path, "r").unwrap_or_else(|_| {
        common::error(&format!("ERROR: Could not open file {}", file_path),
            file!(), line!() as i32, "parse_import");
        unreachable!()
    });

    while !peek_is_eof(&mut imported_file) {
        parse_space_chars(&mut imported_file);
        let scanned = parse_token(peek(&mut imported_file));
        if scanned == tokens_t::VARIABLE {
            let var_name = parse_variable(&mut imported_file);
            if var_name == "def" {
                parse_definition(table, &mut imported_file);
            } else if var_name == "type" {
                parse_type_definition(table, &mut imported_file);
            } else {
                common::error(
                    &format!("Expected a definition in the imported file, but got {}", var_name),
                    file!(), line!() as i32, "parse_import",
                );
            }
        } else {
            break;
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
    let next_token = next_char(in_);
    if parse_token(next_token) != tokens_t::WHITESPACE {
        expect(" ", next_token);
    }

    let next_token = peek(in_);
    if parse_token(next_token) != tokens_t::VARIABLE {
        expect("a type definition", next_token);
    }

    if !is_uppercase(next_token) {
        common::error("Type names must start with an uppercase letter", file!(), line!() as i32, "parse_type_definition");
    }

    let type_name = parse_variable(in_);
    if types_table.table_exists(&type_name) {
        common::error(
            &format!("Type {} was already defined.", type_name),
            file!(), line!() as i32, "parse_type_definition",
        );
    }
    types_table.insert(&type_name, AstNode::default());
}
pub fn parse_type(types_table: &mut hash_table::HashTable, in_: &mut File) -> String {
    let token = next_char(in_);
    if !is_uppercase(token) {
        common::error("Types should start with an uppercase letter.", file!(), line!() as i32, "parse_type");
    }

    let mut type_name = String::new();
    type_name.push(token);

    while is_variable(peek(in_)) {
        type_name.push(next_char(in_));
    }

    if !types_table.table_exists(&type_name) {
        common::error(
            &format!("Type {} was not defined.", type_name),
            file!(), line!() as i32, "parse_type",
        );
    }
    type_name
}
pub fn parse_variable(in_: &mut File) -> String {
    let mut variable_name = String::new();
    while is_variable(peek(in_)) {
        variable_name.push(next_char(in_));
    }
    variable_name
}
pub fn expect(expected: &str, received: char) {
    println!("Syntax Error: Expected {} , received {} ", expected, received);
    std::process::exit(1);
}
pub fn free_ast(_node: &mut AstNode) {
    // Rust handles memory automatically via Drop
}
