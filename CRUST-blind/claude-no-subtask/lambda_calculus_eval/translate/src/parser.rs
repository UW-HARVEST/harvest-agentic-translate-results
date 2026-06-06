use crate::common::{self, AstNode, AstNodeType, AstNodeUnion, Application, LambdaExpression, Variable};
use crate::{hash_table, io as io_mod, reducer};
use std::fs::File;
use std::io::{Seek, SeekFrom};
use std::sync::atomic::{AtomicU64, Ordering};

static ALPHA_COUNTER: AtomicU64 = AtomicU64::new(1);

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
    if node.is_null_sentinel_check() {
        return;
    }
    match node.type_ {
        AstNodeType::LAMBDA_EXPR => {
            if let AstNodeUnion::LambdaExpr(lam) = &node.node {
                print!("(LAMBDA {} : {}", lam.parameter, lam.type_);
                if let Some(body) = &lam.body {
                    print_ast(body);
                }
                print!(") ");
            }
        }
        AstNodeType::APPLICATION => {
            if let AstNodeUnion::Application(app) = &node.node {
                print!("(APP ");
                if let Some(f) = &app.function {
                    print_ast(f);
                }
                if let Some(a) = &app.argument {
                    print_ast(a);
                }
                print!(") ");
            }
        }
        AstNodeType::VAR => {
            if let AstNodeUnion::Variable(v) = &node.node {
                print!("(VAR {} ", v.name);
                if !v.type_.is_empty() {
                    print!(": {}", v.type_);
                }
                print!(")");
            }
        }
        AstNodeType::DEFINITION => {
            if let AstNodeUnion::Variable(v) = &node.node {
                print!("(DEFINITION {}) ", v.name);
            }
        }
    }
}

pub fn is_variable(token: char) -> bool {
    let c = token as u32;
    if c == '_' as u32 {
        return true;
    }
    (c >= 97 && c <= 122) || (c >= 65 && c <= 90)
}

pub fn peek(in_: &mut File) -> char {
    let mut buf = [0u8; 1];
    use std::io::Read;
    match in_.read(&mut buf) {
        Ok(0) => '\u{FFFF}',
        Ok(_) => {
            // seek back one
            let _ = in_.seek(SeekFrom::Current(-1));
            buf[0] as char
        }
        Err(_) => '\u{FFFF}',
    }
}

pub fn peek_print(in_: &mut File, n: usize) {
    let mut buf = vec![0u8; n];
    use std::io::Read;
    let read_n = in_.read(&mut buf).unwrap_or(0);
    let s = String::from_utf8_lossy(&buf[..read_n]);
    print!("{}", s);
    let _ = in_.seek(SeekFrom::Current(-(read_n as i64)));
}

pub fn consume(t: common::tokens_t, in_: &mut File, expected: &str) {
    let c = io_mod::next(in_).unwrap_or('\u{FFFF}');
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
            function: Some(Box::new(reducer::deepcopy(function))),
            argument: Some(Box::new(reducer::deepcopy(argument))),
        }),
    }
}

pub fn create_lambda(variable: &str, body: &AstNode, type_: &str) -> AstNode {
    AstNode {
        type_: AstNodeType::LAMBDA_EXPR,
        node: AstNodeUnion::LambdaExpr(LambdaExpression {
            parameter: variable.to_string(),
            type_: type_.to_string(),
            body: Some(Box::new(reducer::deepcopy(body))),
        }),
    }
}

pub fn alpha_convert(old: &str) -> String {
    let n = ALPHA_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("{}_{}", old, n)
}

pub fn is_used(table: &hash_table::HashTable, variable: &str) -> bool {
    table.table_exists(variable)
}

pub fn parse_space_chars(in_: &mut File) {
    let mut c = peek(in_);
    while c == ' ' || c == '\n' || c == '\t' {
        let _ = io_mod::next(in_);
        c = peek(in_);
    }
}

pub fn parse_lambda(table: &mut hash_table::HashTable, in_: &mut File) -> AstNode {
    if parse_token(peek(in_)) != common::tokens_t::VARIABLE {
        expect("A variable", peek(in_));
    }

    let var = parse_variable(in_);
    let mut new_var: Option<String> = None;
    if is_used(table, &var) {
        if table.search(&var).is_some() {
            let msg = format!(
                "A definition with name {} already exists. Cannot use same name for lambda abstraction.\n",
                var
            );
            common::error(&msg, file!(), line!() as i32, "parse_lambda");
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
        common::error(
            "Lambda abstractions should be typed.",
            file!(),
            line!() as i32,
            "parse_lambda",
        );
    }

    let type_ = parse_type(table, in_);
    consume(common::tokens_t::DOT, in_, ".");

    let mut body = parse_expression(table, in_);

    if let Some(nv) = new_var {
        reducer::replace(&mut body, &var, &nv);
        return create_lambda(&nv, &body, &type_);
    }
    create_lambda(&var, &body, &type_)
}

pub fn parse_expression(table: &mut hash_table::HashTable, in_: &mut File) -> AstNode {
    while parse_token(peek(in_)) == common::tokens_t::WHITESPACE
        || parse_token(peek(in_)) == common::tokens_t::NEWLINE
    {
        let _ = io_mod::next(in_);
    }
    let scanned = parse_token(peek(in_));

    if scanned == common::tokens_t::ERROR {
        // EOF returns ERROR via parse_token; mimic the C behavior of erroring on
        // truly unexpected tokens by returning a sentinel default.
        if peek(in_) == '\u{FFFF}' {
            return AstNode::default();
        }
        println!("Error: {} is  a valid token", peek(in_));
        std::process::exit(1);
    }

    if scanned == common::tokens_t::LAMBDA {
        let _ = io_mod::next(in_);
        return parse_lambda(table, in_);
    } else if scanned == common::tokens_t::L_PAREN {
        let _ = io_mod::next(in_);
        let expr = parse_expression(table, in_);

        let next_token = parse_token(peek(in_));
        // if it is a whitespace, it is a function application
        if next_token == common::tokens_t::WHITESPACE {
            let expr_2 = parse_expression(table, in_);
            let application = AstNode {
                type_: AstNodeType::APPLICATION,
                node: AstNodeUnion::Application(Application {
                    function: Some(Box::new(expr)),
                    argument: Some(Box::new(expr_2)),
                }),
            };
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
            return AstNode::default();
        } else if var_name == "type" {
            parse_type_definition(table, in_);
            if peek(in_) != '\u{FFFF}' {
                return parse_expression(table, in_);
            }
            return AstNode::default();
        }

        let mut type_str = String::new();
        parse_space_chars(in_);
        if !table.table_exists(&var_name) {
            if parse_token(peek(in_)) != common::tokens_t::COLON {
                let msg = format!(
                    "Constant Variable {} is not typed. Please provide a type.\n",
                    var_name
                );
                common::error(&msg, file!(), line!() as i32, "parse_expression");
            }
            consume(common::tokens_t::COLON, in_, ":");
            parse_space_chars(in_);
            type_str = parse_type(table, in_);
        }

        let mut variable = create_variable(&var_name, &type_str);
        if table.search(&var_name).is_some() {
            variable.type_ = AstNodeType::DEFINITION;
        }
        return variable;
    }
    AstNode::default()
}

pub fn parse_import(table: &mut hash_table::HashTable, in_: &mut File) {
    consume(common::tokens_t::WHITESPACE, in_, "a whitespace");
    consume(common::tokens_t::QUOTE, in_, "\"");

    let mut file_path = String::new();
    let mut next_token = io_mod::next(in_).unwrap_or('\u{FFFF}');
    let mut n = parse_token(next_token);

    while n != common::tokens_t::QUOTE {
        if file_path.len() < 99 {
            file_path.push(next_token);
        } else {
            common::error(
                "File path is too long. Please make sure it is less than 100 characters.",
                file!(),
                line!() as i32,
                "parse_import",
            );
        }
        next_token = io_mod::next(in_).unwrap_or('\u{FFFF}');
        n = parse_token(next_token);
    }

    let mut imported_file = match io_mod::get_file(&file_path, "r") {
        Ok(f) => f,
        Err(_) => {
            let msg = format!("ERROR: Could not open file {}\n", file_path);
            common::error(&msg, file!(), line!() as i32, "parse_import");
            return;
        }
    };

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
                let msg = format!(
                    "Expected a definition in the imported file, but got {}\n",
                    var_name
                );
                common::error(&msg, file!(), line!() as i32, "parse_import");
            }
        } else {
            // skip a char to make progress
            let _ = io_mod::next(&mut imported_file);
        }
    }
}

pub fn parse_definition(table: &mut hash_table::HashTable, in_: &mut File) {
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

pub fn parse_type_definition(types_table: &mut hash_table::HashTable, in_: &mut File) {
    let next_token = io_mod::next(in_).unwrap_or('\u{FFFF}');
    let n = parse_token(next_token);
    if n != common::tokens_t::WHITESPACE {
        expect(" ", next_token);
    }

    let next_token = peek(in_);
    let n = parse_token(next_token);
    if n != common::tokens_t::VARIABLE {
        expect("a type definition", next_token);
    }

    if !is_uppercase(next_token) {
        common::error(
            "Type names must start with an uppercase letter",
            file!(),
            line!() as i32,
            "parse_type_definition",
        );
    }

    let type_name = parse_variable(in_);
    if types_table.table_exists(&type_name) {
        let msg = format!("Type {} was already defined.\n", type_name);
        common::error(&msg, file!(), line!() as i32, "parse_type_definition");
    }
    types_table.insert(&type_name, AstNode::default());
}

pub fn parse_type(types_table: &mut hash_table::HashTable, in_: &mut File) -> String {
    let mut type_name = String::new();
    let token = io_mod::next(in_).unwrap_or('\u{FFFF}');

    if !is_uppercase(token) {
        common::error(
            "Types should start with an uppercase letter.",
            file!(),
            line!() as i32,
            "parse_type",
        );
    }

    type_name.push(token);

    while is_variable(peek(in_)) {
        let c = io_mod::next(in_).unwrap_or('\u{FFFF}');
        type_name.push(c);
    }

    if !types_table.table_exists(&type_name) {
        let msg = format!("Type {} was not defined.\n", type_name);
        common::error(&msg, file!(), line!() as i32, "parse_type");
    }
    type_name
}

pub fn parse_variable(in_: &mut File) -> String {
    let mut variable_name = String::new();
    while is_variable(peek(in_)) {
        let c = io_mod::next(in_).unwrap_or('\u{FFFF}');
        variable_name.push(c);
    }
    variable_name
}

pub fn expect(expected: &str, received: char) {
    println!("Syntax Error: Expected {} , received {} ", expected, received);
    std::process::exit(1);
}

pub fn free_ast(_node: &mut AstNode) {
    // No-op in Rust: ownership/Drop handles cleanup.
}

// Helper trait extension to keep the existing AstNode API minimal.
trait AstNodeNullCheck {
    fn is_null_sentinel_check(&self) -> bool;
}

impl AstNodeNullCheck for AstNode {
    fn is_null_sentinel_check(&self) -> bool {
        if let AstNodeUnion::Variable(v) = &self.node {
            self.type_ == AstNodeType::VAR && v.name == "\u{0}__NULL_SENTINEL__\u{0}"
        } else {
            false
        }
    }
}
