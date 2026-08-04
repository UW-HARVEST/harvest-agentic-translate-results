pub const OBJECT_NUMBER: usize = 50;
pub const BLOCK_SIZE: usize = std::mem::size_of::<Object>();
pub const MEMORY_SIZE: usize = BLOCK_SIZE * OBJECT_NUMBER;
pub const FREE_BITMAP_SIZE: usize = MEMORY_SIZE / BLOCK_SIZE;
pub const MAX_BINDINGS: usize = 10;
pub const MAX_SYMBOL_NAME_LENGTH: usize = 20;
#[derive(Debug, Clone, Copy)]
pub enum ConsCellType {
Cell,
Nil,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Digit,
    LParen,
    RParen,
    Symbol,
    String,
    True,
    False,
    Eof,
    Quote,
}
#[derive(Debug)]
pub struct Token {
pub kind: TokenKind,
pub next: Option<Box<Token>>,
pub val: i32,
pub str: String,
}
#[derive(Debug)]
pub struct ParseState {
pub token: Option<Box<Token>>,
pub pos: i32,
}
#[derive(Debug)]
pub struct ProgramNode {
pub expressions: Option<Box<ExpressionList>>,
}
#[derive(Debug, Clone, Copy)]
pub enum ExpressionType {
Literal,
Symbol,
List,
SymbolicExp,
}
#[derive(Debug)]
pub struct ExpressionList {
pub expression: Option<Box<ExpressionNode>>,
pub next: Option<Box<ExpressionList>>,
}
#[derive(Debug)]
pub struct ExpressionNode {
pub type_: ExpressionType,
pub data: ExpressionData,
}
#[derive(Debug)]
pub enum ExpressionData {
SymbolicExp(Option<Box<SymbolicExpNode>>),
List(Option<Box<ListNode>>),
Literal(Option<Box<LiteralNode>>),
Symbol(Option<Box<SymbolNode>>),
}
#[derive(Debug)]
pub struct SymbolicExpNode {
pub expressions: Option<Box<ExpressionList>>,
}
#[derive(Debug)]
pub struct ListNode {
pub expressions: Option<Box<ExpressionList>>,
}
#[derive(Debug, Clone, Copy)]
pub enum LiteralType {
Integer,
String,
Boolean,
}
#[derive(Debug)]
pub struct LiteralNode {
pub type_: LiteralType,
pub value: LiteralValue,
}
#[derive(Debug)]
pub enum LiteralValue {
IntValue(i32),
BooleanValue(bool),
StringValue(String),
}
#[derive(Debug)]
pub struct SymbolNode {
pub symbol_name: String,
}
#[derive(Debug)]
pub struct ParseResult {
pub program: Option<Box<ProgramNode>>,
}
#[derive(Debug, Clone, Copy)]
pub enum ObjectType {
Integer,
String,
Bool,
List,
Nil,
Function,
}
#[derive(Debug)]
pub struct ConsCell {
pub type_: ConsCellType,
pub car: Option<Box<Object>>,
pub cdr: Option<Box<Object>>,
}
#[derive(Debug)]
pub struct Function {
pub param_symbol_names: Vec<String>,
pub body: Option<Box<ExpressionNode>>,
}
#[derive(Debug)]
pub struct Object {
pub marked: bool,
pub type_: ObjectType,
pub value: ObjectValue,
}
#[derive(Debug)]
pub enum ObjectValue {
IntValue(i32),
StringValue(String),
BoolValue(i32),
ListValue(Option<Box<ConsCell>>),
FunctionValue(Option<Box<Function>>),
}
#[derive(Debug)]
pub struct Binding {
pub symbol_name: String,
pub value: Option<Box<Object>>,
}
#[derive(Debug)]
pub struct Env {
pub bindings: [Binding; MAX_BINDINGS],
pub parent: Option<Box<Env>>,
}
#[derive(Debug)]
pub struct ObjectStack {
pub objects: [Option<Box<Object>>; OBJECT_NUMBER],
pub top: i32,
}
#[derive(Debug)]
pub struct AllocatorContext {
pub gc_less_mode: i32,
pub stack: Option<Box<ObjectStack>>,
pub memory_pool: Option<Box<Object>>,
pub free_bitmap: [u8; FREE_BITMAP_SIZE],
}

// =================================================
// Helpers
// =================================================

fn is_op_char(c: char) -> bool {
    matches!(
        c,
        '+' | '-' | '*' | '/' | '%' | '|' | '&' | '=' | '<' | '>'
    )
}

fn default_object() -> Object {
    Object {
        marked: false,
        type_: ObjectType::Nil,
        value: ObjectValue::IntValue(0),
    }
}

fn nil_object() -> Object {
    Object {
        marked: false,
        type_: ObjectType::Nil,
        value: ObjectValue::IntValue(0),
    }
}

fn empty_binding() -> Binding {
    Binding {
        symbol_name: String::new(),
        value: None,
    }
}

fn default_expression_node() -> ExpressionNode {
    ExpressionNode {
        type_: ExpressionType::Literal,
        data: ExpressionData::Literal(None),
    }
}

// Deep clone helpers

fn clone_object(obj: &Object) -> Object {
    Object {
        marked: obj.marked,
        type_: obj.type_,
        value: clone_object_value(&obj.value),
    }
}

fn clone_object_value(v: &ObjectValue) -> ObjectValue {
    match v {
        ObjectValue::IntValue(n) => ObjectValue::IntValue(*n),
        ObjectValue::StringValue(s) => ObjectValue::StringValue(s.clone()),
        ObjectValue::BoolValue(b) => ObjectValue::BoolValue(*b),
        ObjectValue::ListValue(c) => ObjectValue::ListValue(
            c.as_ref().map(|c| Box::new(clone_cons_cell(c))),
        ),
        ObjectValue::FunctionValue(f) => ObjectValue::FunctionValue(
            f.as_ref().map(|f| Box::new(clone_function(f))),
        ),
    }
}

fn clone_cons_cell(c: &ConsCell) -> ConsCell {
    ConsCell {
        type_: c.type_,
        car: c.car.as_ref().map(|o| Box::new(clone_object(o))),
        cdr: c.cdr.as_ref().map(|o| Box::new(clone_object(o))),
    }
}

fn clone_function(f: &Function) -> Function {
    Function {
        param_symbol_names: f.param_symbol_names.clone(),
        body: f
            .body
            .as_ref()
            .map(|b| Box::new(clone_expression_node(b))),
    }
}

fn clone_expression_node(e: &ExpressionNode) -> ExpressionNode {
    ExpressionNode {
        type_: e.type_,
        data: clone_expression_data(&e.data),
    }
}

fn clone_expression_data(d: &ExpressionData) -> ExpressionData {
    match d {
        ExpressionData::SymbolicExp(s) => ExpressionData::SymbolicExp(s.as_ref().map(|s| {
            Box::new(SymbolicExpNode {
                expressions: s
                    .expressions
                    .as_ref()
                    .map(|el| Box::new(clone_expression_list(el))),
            })
        })),
        ExpressionData::List(l) => ExpressionData::List(l.as_ref().map(|l| {
            Box::new(ListNode {
                expressions: l
                    .expressions
                    .as_ref()
                    .map(|el| Box::new(clone_expression_list(el))),
            })
        })),
        ExpressionData::Literal(l) => {
            ExpressionData::Literal(l.as_ref().map(|l| Box::new(clone_literal_node(l))))
        }
        ExpressionData::Symbol(s) => ExpressionData::Symbol(s.as_ref().map(|s| {
            Box::new(SymbolNode {
                symbol_name: s.symbol_name.clone(),
            })
        })),
    }
}

fn clone_expression_list(e: &ExpressionList) -> ExpressionList {
    ExpressionList {
        expression: e
            .expression
            .as_ref()
            .map(|e| Box::new(clone_expression_node(e))),
        next: e.next.as_ref().map(|n| Box::new(clone_expression_list(n))),
    }
}

fn clone_literal_node(l: &LiteralNode) -> LiteralNode {
    LiteralNode {
        type_: l.type_,
        value: clone_literal_value(&l.value),
    }
}

fn clone_literal_value(v: &LiteralValue) -> LiteralValue {
    match v {
        LiteralValue::IntValue(n) => LiteralValue::IntValue(*n),
        LiteralValue::BooleanValue(b) => LiteralValue::BooleanValue(*b),
        LiteralValue::StringValue(s) => LiteralValue::StringValue(s.clone()),
    }
}

fn clone_binding(b: &Binding) -> Binding {
    Binding {
        symbol_name: b.symbol_name.clone(),
        value: b.value.as_ref().map(|v| Box::new(clone_object(v))),
    }
}

fn clone_env(env: &Env) -> Env {
    Env {
        bindings: std::array::from_fn(|i| clone_binding(&env.bindings[i])),
        parent: env.parent.as_ref().map(|p| Box::new(clone_env(p))),
    }
}

// Append to expression list
fn append_to_expression_list(
    list: &mut Option<Box<ExpressionList>>,
    expr: ExpressionNode,
) {
    let new_node = ExpressionList {
        expression: Some(Box::new(expr)),
        next: None,
    };
    if list.is_none() {
        *list = Some(Box::new(new_node));
        return;
    }
    let mut cur = list.as_mut().unwrap();
    while cur.next.is_some() {
        cur = cur.next.as_mut().unwrap();
    }
    cur.next = Some(Box::new(new_node));
}

// =================================================
// Tokenizer
// =================================================

pub fn match_token(state: &mut ParseState, kind: TokenKind) -> i32 {
    match &state.token {
        Some(t) => {
            if t.kind == kind {
                1
            } else {
                0
            }
        }
        None => 0,
    }
}

fn match_kind(state: &ParseState, kind: TokenKind) -> bool {
    match &state.token {
        Some(t) => t.kind == kind,
        None => false,
    }
}

pub fn next(source: &str, state: &mut ParseState) {
    let bytes = source.as_bytes();

    // Skip whitespaces
    while (state.pos as usize) < bytes.len() {
        let c = bytes[state.pos as usize];
        if (c as char).is_ascii_whitespace() {
            state.pos += 1;
        } else {
            break;
        }
    }

    let mut new_token = Token {
        kind: TokenKind::Eof,
        next: None,
        val: 0,
        str: String::new(),
    };

    let pos = state.pos as usize;
    if pos >= bytes.len() {
        new_token.kind = TokenKind::Eof;
        new_token.str = "\0".to_string();
        state.token = Some(Box::new(new_token));
        return;
    }

    let c = bytes[pos] as char;

    if c == '(' {
        new_token.kind = TokenKind::LParen;
        new_token.str = "(".to_string();
        state.pos += 1;
    } else if c == ')' {
        new_token.kind = TokenKind::RParen;
        new_token.str = ")".to_string();
        state.pos += 1;
    } else if c == '\'' {
        new_token.kind = TokenKind::Quote;
        new_token.str = "'".to_string();
        state.pos += 1;
    } else if c.is_ascii_alphabetic() || is_op_char(c) {
        // tokenize symbol
        let start = state.pos as usize;
        while (state.pos as usize) < bytes.len() {
            let cc = bytes[state.pos as usize] as char;
            if cc.is_ascii_alphanumeric() || is_op_char(cc) {
                state.pos += 1;
            } else {
                break;
            }
        }
        let end = state.pos as usize;
        let s = std::str::from_utf8(&bytes[start..end])
            .unwrap_or("")
            .to_string();
        if s == "true" {
            new_token.kind = TokenKind::True;
        } else if s == "false" {
            new_token.kind = TokenKind::False;
        } else {
            new_token.kind = TokenKind::Symbol;
            new_token.str = s;
        }
    } else if c.is_ascii_digit() {
        // tokenize digit
        let start = state.pos as usize;
        while (state.pos as usize) < bytes.len() {
            let cc = bytes[state.pos as usize] as char;
            if cc.is_ascii_digit() {
                state.pos += 1;
            } else {
                break;
            }
        }
        let end = state.pos as usize;
        let s = std::str::from_utf8(&bytes[start..end]).unwrap_or("");
        let val: i32 = s.parse().unwrap_or(0);
        new_token.kind = TokenKind::Digit;
        new_token.val = val;
    } else if c == '"' {
        // tokenize string
        state.pos += 1; // skip opening quote
        let start = state.pos as usize;
        while (state.pos as usize) < bytes.len() {
            let cc = bytes[state.pos as usize];
            if cc == b'"' {
                break;
            }
            state.pos += 1;
        }
        let end = state.pos as usize;
        new_token.kind = TokenKind::String;
        new_token.str = std::str::from_utf8(&bytes[start..end])
            .unwrap_or("")
            .to_string();
        if (state.pos as usize) < bytes.len() && bytes[state.pos as usize] == b'"' {
            state.pos += 1;
        }
    } else if c == ';' {
        // tokenize comment
        while (state.pos as usize) < bytes.len() {
            let cc = bytes[state.pos as usize];
            if cc == b'\n' {
                break;
            }
            state.pos += 1;
        }
        next(source, state);
        return;
    } else {
        eprintln!("Unexpected token: {}", c);
        std::process::exit(1);
    }

    state.token = Some(Box::new(new_token));
}

// =================================================
// Parser
// =================================================

fn parse_symbolic_expression(
    source: &str,
    state: &mut ParseState,
    expression: &mut ExpressionNode,
) {
    let mut symbolic_exp = SymbolicExpNode { expressions: None };
    next(source, state); // eat '('
    while !match_kind(state, TokenKind::RParen) {
        let mut item = default_expression_node();
        parse_expression(source, state, &mut item);
        append_to_expression_list(&mut symbolic_exp.expressions, item);
    }
    next(source, state); // eat ')'
    expression.type_ = ExpressionType::SymbolicExp;
    expression.data = ExpressionData::SymbolicExp(Some(Box::new(symbolic_exp)));
}

fn parse_list_expression(
    source: &str,
    state: &mut ParseState,
    expression: &mut ExpressionNode,
) {
    let mut list = ListNode { expressions: None };
    next(source, state); // eat quote
    next(source, state); // eat '('
    while !match_kind(state, TokenKind::RParen) {
        let mut item = default_expression_node();
        parse_expression(source, state, &mut item);
        append_to_expression_list(&mut list.expressions, item);
    }
    next(source, state); // eat ')'
    expression.type_ = ExpressionType::List;
    expression.data = ExpressionData::List(Some(Box::new(list)));
}

fn parse_symbol_expression(
    source: &str,
    state: &mut ParseState,
    expression: &mut ExpressionNode,
) {
    let symbol_name = state
        .token
        .as_ref()
        .map(|t| t.str.clone())
        .unwrap_or_default();
    expression.type_ = ExpressionType::Symbol;
    expression.data = ExpressionData::Symbol(Some(Box::new(SymbolNode { symbol_name })));
    next(source, state);
}

fn parse_literal_expression(
    source: &str,
    state: &mut ParseState,
    expression: &mut ExpressionNode,
) {
    if match_kind(state, TokenKind::Digit) {
        let val = state.token.as_ref().map(|t| t.val).unwrap_or(0);
        expression.type_ = ExpressionType::Literal;
        expression.data = ExpressionData::Literal(Some(Box::new(LiteralNode {
            type_: LiteralType::Integer,
            value: LiteralValue::IntValue(val),
        })));
        next(source, state);
    } else if match_kind(state, TokenKind::String) {
        let s = state
            .token
            .as_ref()
            .map(|t| t.str.clone())
            .unwrap_or_default();
        expression.type_ = ExpressionType::Literal;
        expression.data = ExpressionData::Literal(Some(Box::new(LiteralNode {
            type_: LiteralType::String,
            value: LiteralValue::StringValue(s),
        })));
        next(source, state);
    } else if match_kind(state, TokenKind::True) {
        expression.type_ = ExpressionType::Literal;
        expression.data = ExpressionData::Literal(Some(Box::new(LiteralNode {
            type_: LiteralType::Boolean,
            value: LiteralValue::BooleanValue(true),
        })));
        next(source, state);
    } else if match_kind(state, TokenKind::False) {
        expression.type_ = ExpressionType::Literal;
        expression.data = ExpressionData::Literal(Some(Box::new(LiteralNode {
            type_: LiteralType::Boolean,
            value: LiteralValue::BooleanValue(false),
        })));
        next(source, state);
    } else {
        let s = state
            .token
            .as_ref()
            .map(|t| t.str.clone())
            .unwrap_or_default();
        eprintln!("Unexpected token: {}", s);
        std::process::exit(1);
    }
}

fn parse_expression(
    source: &str,
    state: &mut ParseState,
    expression: &mut ExpressionNode,
) {
    if match_kind(state, TokenKind::LParen) {
        parse_symbolic_expression(source, state, expression);
    } else if match_kind(state, TokenKind::Quote) {
        parse_list_expression(source, state, expression);
    } else if match_kind(state, TokenKind::Symbol) {
        parse_symbol_expression(source, state, expression);
    } else if match_kind(state, TokenKind::Digit)
        || match_kind(state, TokenKind::String)
        || match_kind(state, TokenKind::True)
        || match_kind(state, TokenKind::False)
    {
        parse_literal_expression(source, state, expression);
    } else {
        let s = state
            .token
            .as_ref()
            .map(|t| t.str.clone())
            .unwrap_or_default();
        eprintln!("Unexpected token: {}", s);
        std::process::exit(1);
    }
}

fn parse_program(source: &str, state: &mut ParseState, result: &mut ParseResult) {
    next(source, state);
    let mut program = ProgramNode { expressions: None };
    while !match_kind(state, TokenKind::Eof) {
        let mut item = default_expression_node();
        parse_expression(source, state, &mut item);
        append_to_expression_list(&mut program.expressions, item);
    }
    result.program = Some(Box::new(program));
}

pub fn parse(source: &str, state: &mut ParseState, result: &mut ParseResult) {
    parse_program(source, state, result);
}

// =================================================
// GC / Allocator
// =================================================

pub fn init_allocator() -> AllocatorContext {
    AllocatorContext {
        gc_less_mode: 0,
        stack: Some(Box::new(ObjectStack {
            objects: std::array::from_fn(|_| None),
            top: -1,
        })),
        memory_pool: None,
        free_bitmap: [0u8; FREE_BITMAP_SIZE],
    }
}

pub fn allocate(_context: &mut AllocatorContext, _env: &mut Env) -> Option<Box<Object>> {
    Some(Box::new(default_object()))
}

// =================================================
// Defined functions / helpers
// =================================================

fn bool_val(obj: &Object) -> bool {
    match obj.type_ {
        ObjectType::Bool => match &obj.value {
            ObjectValue::BoolValue(b) => *b != 0,
            _ => false,
        },
        ObjectType::Nil => false,
        _ => true,
    }
}

fn eq_obj(op1: &Object, op2: &Object) -> bool {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => match (&op1.value, &op2.value) {
            (ObjectValue::IntValue(a), ObjectValue::IntValue(b)) => a == b,
            _ => false,
        },
        (ObjectType::String, ObjectType::String) => match (&op1.value, &op2.value) {
            (ObjectValue::StringValue(a), ObjectValue::StringValue(b)) => a == b,
            _ => false,
        },
        (ObjectType::Bool, ObjectType::Bool) => match (&op1.value, &op2.value) {
            (ObjectValue::BoolValue(a), ObjectValue::BoolValue(b)) => a == b,
            _ => false,
        },
        (ObjectType::List, ObjectType::List) => false, // Pointer comparison in C; in Rust, we don't share pointers
        (ObjectType::Nil, ObjectType::Nil) => true,
        _ => false,
    }
}

fn is_last_cons_cell(c: &ConsCell) -> bool {
    matches!(
        c.cdr.as_ref().map(|o| o.type_),
        Some(ObjectType::Nil)
    )
}

pub fn stringify_object(obj: &Object) -> String {
    match obj.type_ {
        ObjectType::Integer => match &obj.value {
            ObjectValue::IntValue(n) => n.to_string(),
            _ => "0".to_string(),
        },
        ObjectType::String => match &obj.value {
            ObjectValue::StringValue(s) => s.clone(),
            _ => String::new(),
        },
        ObjectType::Bool => match &obj.value {
            ObjectValue::BoolValue(b) => {
                if *b != 0 {
                    "T".to_string()
                } else {
                    "F".to_string()
                }
            }
            _ => "F".to_string(),
        },
        ObjectType::List => {
            let mut s = String::from("(");
            let mut first = true;
            let mut current = match &obj.value {
                ObjectValue::ListValue(c) => c.as_deref(),
                _ => None,
            };
            while let Some(cell) = current {
                if !first {
                    s.push(' ');
                }
                first = false;
                if let Some(car) = &cell.car {
                    s.push_str(&stringify_object(car));
                }
                if is_last_cons_cell(cell) {
                    break;
                }
                current = match cell.cdr.as_deref() {
                    Some(o) => match &o.value {
                        ObjectValue::ListValue(c) => c.as_deref(),
                        _ => None,
                    },
                    None => None,
                };
            }
            s.push(')');
            s
        }
        ObjectType::Nil => "nil".to_string(),
        ObjectType::Function => "<function>".to_string(),
    }
}

fn defined_function_add(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            let a = match &op1.value {
                ObjectValue::IntValue(n) => *n,
                _ => 0,
            };
            let b = match &op2.value {
                ObjectValue::IntValue(n) => *n,
                _ => 0,
            };
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(a.wrapping_add(b));
        }
        (ObjectType::String, ObjectType::String) => {
            let a = match &op1.value {
                ObjectValue::StringValue(s) => s.clone(),
                _ => String::new(),
            };
            let b = match &op2.value {
                ObjectValue::StringValue(s) => s.clone(),
                _ => String::new(),
            };
            evaluated.type_ = ObjectType::String;
            evaluated.value = ObjectValue::StringValue(a + &b);
        }
        _ => {
            eprintln!("Type error: operands for + must be integers or strings.");
            std::process::exit(1);
        }
    }
}

fn defined_function_sub(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            let a = match &op1.value {
                ObjectValue::IntValue(n) => *n,
                _ => 0,
            };
            let b = match &op2.value {
                ObjectValue::IntValue(n) => *n,
                _ => 0,
            };
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(a.wrapping_sub(b));
        }
        _ => {
            eprintln!("Type error: operands for - must be integers.");
            std::process::exit(1);
        }
    }
}

fn defined_function_mul(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            let a = match &op1.value {
                ObjectValue::IntValue(n) => *n,
                _ => 0,
            };
            let b = match &op2.value {
                ObjectValue::IntValue(n) => *n,
                _ => 0,
            };
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(a.wrapping_mul(b));
        }
        _ => {
            eprintln!("Type error: operands for * must be integers.");
            std::process::exit(1);
        }
    }
}

fn defined_function_div(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            let a = match &op1.value {
                ObjectValue::IntValue(n) => *n,
                _ => 0,
            };
            let b = match &op2.value {
                ObjectValue::IntValue(n) => *n,
                _ => 0,
            };
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(if b == 0 { 0 } else { a / b });
        }
        _ => {
            eprintln!("Type error: operands for / must be integers.");
            std::process::exit(1);
        }
    }
}

fn defined_function_mod(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            let a = match &op1.value {
                ObjectValue::IntValue(n) => *n,
                _ => 0,
            };
            let b = match &op2.value {
                ObjectValue::IntValue(n) => *n,
                _ => 0,
            };
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(if b == 0 { 0 } else { a % b });
        }
        _ => {
            eprintln!("Type error: operands for % must be integers.");
            std::process::exit(1);
        }
    }
}

fn defined_function_lt(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            let a = match &op1.value {
                ObjectValue::IntValue(n) => *n,
                _ => 0,
            };
            let b = match &op2.value {
                ObjectValue::IntValue(n) => *n,
                _ => 0,
            };
            evaluated.type_ = ObjectType::Bool;
            evaluated.value = ObjectValue::BoolValue(if a < b { 1 } else { 0 });
        }
        _ => {
            eprintln!("Type error: operands for < must be integers.");
            std::process::exit(1);
        }
    }
}

fn defined_function_gt(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            let a = match &op1.value {
                ObjectValue::IntValue(n) => *n,
                _ => 0,
            };
            let b = match &op2.value {
                ObjectValue::IntValue(n) => *n,
                _ => 0,
            };
            evaluated.type_ = ObjectType::Bool;
            evaluated.value = ObjectValue::BoolValue(if a > b { 1 } else { 0 });
        }
        _ => {
            eprintln!("Type error: operands for > must be integers.");
            std::process::exit(1);
        }
    }
}

fn defined_function_eq(op1: &Object, op2: &Object, evaluated: &mut Object) {
    evaluated.type_ = ObjectType::Bool;
    evaluated.value = ObjectValue::BoolValue(if eq_obj(op1, op2) { 1 } else { 0 });
}

fn defined_function_not(op: &Object, evaluated: &mut Object) {
    if !matches!(op.type_, ObjectType::Bool) {
        eprintln!("Type error: not operand must be boolean.");
        std::process::exit(1);
    }
    let b = match &op.value {
        ObjectValue::BoolValue(b) => *b,
        _ => 0,
    };
    evaluated.type_ = ObjectType::Bool;
    evaluated.value = ObjectValue::BoolValue(if b != 0 { 0 } else { 1 });
}

fn defined_function_car(op: &Object, evaluated: &mut Object) {
    if !matches!(op.type_, ObjectType::List) {
        eprintln!("Type error: car operand must be list.");
        std::process::exit(1);
    }
    if let ObjectValue::ListValue(Some(c)) = &op.value {
        if let Some(car) = &c.car {
            *evaluated = clone_object(car);
        }
    }
}

fn defined_function_cdr(op: &Object, evaluated: &mut Object) {
    if !matches!(op.type_, ObjectType::List) {
        eprintln!("Type error: cdr operand must be list.");
        std::process::exit(1);
    }
    if let ObjectValue::ListValue(Some(c)) = &op.value {
        if let Some(cdr) = &c.cdr {
            *evaluated = clone_object(cdr);
        }
    }
}

fn defined_function_cons(op1: &Object, op2: &Object, evaluated: &mut Object) {
    evaluated.type_ = ObjectType::List;
    let car = Some(Box::new(clone_object(op1)));
    let (cell_type, cdr) = match op2.type_ {
        ObjectType::List => (
            ConsCellType::Cell,
            Some(Box::new(clone_object(op2))),
        ),
        ObjectType::Nil => (
            ConsCellType::Nil,
            Some(Box::new(clone_object(op2))),
        ),
        _ => {
            // Wrap op2 as a list element with NIL terminator
            let inner_cell = ConsCell {
                type_: ConsCellType::Cell,
                car: Some(Box::new(clone_object(op2))),
                cdr: Some(Box::new(nil_object())),
            };
            let cdr_obj = Object {
                marked: false,
                type_: ObjectType::List,
                value: ObjectValue::ListValue(Some(Box::new(inner_cell))),
            };
            (ConsCellType::Cell, Some(Box::new(cdr_obj)))
        }
    };
    let cell = ConsCell {
        type_: cell_type,
        car,
        cdr,
    };
    evaluated.value = ObjectValue::ListValue(Some(Box::new(cell)));
}

fn defined_function_split(op1: &Object, op2: &Object, evaluated: &mut Object) {
    let s1 = match (&op1.type_, &op1.value) {
        (ObjectType::String, ObjectValue::StringValue(s)) => s.clone(),
        _ => {
            eprintln!("Type error: split first operand must be string.");
            std::process::exit(1);
        }
    };
    let s2 = match (&op2.type_, &op2.value) {
        (ObjectType::String, ObjectValue::StringValue(s)) => s.clone(),
        _ => {
            eprintln!("Type error: split second operand must be string.");
            std::process::exit(1);
        }
    };

    let parts: Vec<String> = if s2.is_empty() {
        s1.chars().map(|c| c.to_string()).collect()
    } else {
        s1.split(&s2)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    };

    if parts.is_empty() {
        evaluated.type_ = ObjectType::Nil;
        evaluated.value = ObjectValue::IntValue(0);
        return;
    }

    let n = parts.len();
    let mut current_cell: Option<ConsCell> = None;
    for (i, part) in parts.into_iter().enumerate().rev() {
        let is_last = i == n - 1;
        let cdr_obj = if is_last {
            nil_object()
        } else {
            Object {
                marked: false,
                type_: ObjectType::List,
                value: ObjectValue::ListValue(current_cell.take().map(Box::new)),
            }
        };
        let car_obj = Object {
            marked: false,
            type_: ObjectType::String,
            value: ObjectValue::StringValue(part),
        };
        let cell = ConsCell {
            type_: if is_last {
                ConsCellType::Nil
            } else {
                ConsCellType::Cell
            },
            car: Some(Box::new(car_obj)),
            cdr: Some(Box::new(cdr_obj)),
        };
        current_cell = Some(cell);
    }
    evaluated.type_ = ObjectType::List;
    evaluated.value = ObjectValue::ListValue(current_cell.map(Box::new));
}

fn defined_function_list_ref(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if !matches!(op1.type_, ObjectType::List) {
        eprintln!("Type error: list-ref first operand must be list.");
        std::process::exit(1);
    }
    let index = match (&op2.type_, &op2.value) {
        (ObjectType::Integer, ObjectValue::IntValue(n)) => *n,
        _ => {
            eprintln!("Type error: list-ref second operand must be integer.");
            std::process::exit(1);
        }
    };

    let mut current = match &op1.value {
        ObjectValue::ListValue(c) => c.as_deref(),
        _ => None,
    };
    for _ in 0..index {
        match current {
            Some(cell) => match cell.cdr.as_deref() {
                Some(o) => {
                    if matches!(o.type_, ObjectType::Nil) {
                        eprintln!("Index out of range.");
                        std::process::exit(1);
                    }
                    current = match &o.value {
                        ObjectValue::ListValue(c) => c.as_deref(),
                        _ => None,
                    };
                }
                None => {
                    eprintln!("Index out of range.");
                    std::process::exit(1);
                }
            },
            None => {
                eprintln!("Index out of range.");
                std::process::exit(1);
            }
        }
    }
    if let Some(cell) = current {
        if let Some(car) = &cell.car {
            *evaluated = clone_object(car);
        }
    }
}

fn defined_function_remove_whitespaces(op: &Object, evaluated: &mut Object) {
    let s = match (&op.type_, &op.value) {
        (ObjectType::String, ObjectValue::StringValue(s)) => s.clone(),
        _ => {
            eprintln!("Type error: remove-whitespaces operand must be string.");
            std::process::exit(1);
        }
    };
    let result: String = s.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    evaluated.type_ = ObjectType::String;
    evaluated.value = ObjectValue::StringValue(result);
}

fn defined_function_pop(op: &Object, evaluated: &mut Object) {
    if matches!(op.type_, ObjectType::Nil) {
        evaluated.type_ = ObjectType::Nil;
        evaluated.value = ObjectValue::IntValue(0);
        return;
    }
    if !matches!(op.type_, ObjectType::List) {
        eprintln!("Type error: pop operand must be list.");
        std::process::exit(1);
    }
    let mut current = match &op.value {
        ObjectValue::ListValue(c) => c.as_deref(),
        _ => None,
    };
    while let Some(cell) = current {
        if is_last_cons_cell(cell) {
            if let Some(car) = &cell.car {
                *evaluated = clone_object(car);
            }
            return;
        }
        current = match cell.cdr.as_deref() {
            Some(o) => match &o.value {
                ObjectValue::ListValue(c) => c.as_deref(),
                _ => None,
            },
            None => None,
        };
    }
}

fn defined_function_length(op: &Object, evaluated: &mut Object) {
    match op.type_ {
        ObjectType::Nil => {
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(0);
        }
        ObjectType::List => {
            let mut len = 0;
            let mut current = match &op.value {
                ObjectValue::ListValue(c) => c.as_deref(),
                _ => None,
            };
            while let Some(cell) = current {
                len += 1;
                if is_last_cons_cell(cell) {
                    break;
                }
                current = match cell.cdr.as_deref() {
                    Some(o) => match &o.value {
                        ObjectValue::ListValue(c) => c.as_deref(),
                        _ => None,
                    },
                    None => None,
                };
            }
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(len);
        }
        ObjectType::String => {
            let len = match &op.value {
                ObjectValue::StringValue(s) => s.len() as i32,
                _ => 0,
            };
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(len);
        }
        _ => {
            eprintln!("Type error: length operand must be list or string.");
            std::process::exit(1);
        }
    }
}

fn defined_function_is_int_string(op: &Object, evaluated: &mut Object) {
    let result = match (&op.type_, &op.value) {
        (ObjectType::String, ObjectValue::StringValue(s)) => {
            !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
        }
        _ => false,
    };
    evaluated.type_ = ObjectType::Bool;
    evaluated.value = ObjectValue::BoolValue(if result { 1 } else { 0 });
}

fn defined_function_parse_int(op: &Object, evaluated: &mut Object) {
    let s = match (&op.type_, &op.value) {
        (ObjectType::String, ObjectValue::StringValue(s)) => s.clone(),
        _ => {
            eprintln!("Type error: parse-int operand must be string.");
            std::process::exit(1);
        }
    };
    if !s.chars().all(|c| c.is_ascii_digit()) {
        eprintln!("Type error: parse-int operand must be string of digits.");
        std::process::exit(1);
    }
    let n: i32 = s.parse().unwrap_or(0);
    evaluated.type_ = ObjectType::Integer;
    evaluated.value = ObjectValue::IntValue(n);
}

fn defined_function_string_ref(op1: &Object, op2: &Object, evaluated: &mut Object) {
    let s = match (&op1.type_, &op1.value) {
        (ObjectType::String, ObjectValue::StringValue(s)) => s.clone(),
        _ => {
            eprintln!("Type error: string-ref first operand must be string.");
            std::process::exit(1);
        }
    };
    let index = match (&op2.type_, &op2.value) {
        (ObjectType::Integer, ObjectValue::IntValue(n)) => *n,
        _ => {
            eprintln!("Type error: string-ref second operand must be integer.");
            std::process::exit(1);
        }
    };
    if index < 0 || index as usize >= s.len() {
        eprintln!("Index out of range.");
        std::process::exit(1);
    }
    let bytes = s.as_bytes();
    let ch = bytes[index as usize] as char;
    evaluated.type_ = ObjectType::String;
    evaluated.value = ObjectValue::StringValue(ch.to_string());
}

// =================================================
// Env helpers
// =================================================

fn set_object_to_env(env: &mut Env, symbol_name: &str, obj: Box<Object>) {
    // Search existing
    for i in 0..MAX_BINDINGS {
        if env.bindings[i].symbol_name.is_empty() {
            // empty slot - add here
            env.bindings[i].symbol_name = symbol_name.to_string();
            env.bindings[i].value = Some(obj);
            return;
        }
        if env.bindings[i].symbol_name == symbol_name {
            env.bindings[i].value = Some(obj);
            return;
        }
    }
}

fn lookup_in_env(env: &Env, symbol_name: &str) -> Option<Object> {
    for binding in env.bindings.iter() {
        if binding.symbol_name.is_empty() {
            break;
        }
        if binding.symbol_name == symbol_name {
            return binding.value.as_ref().map(|v| clone_object(v));
        }
    }
    if let Some(parent) = &env.parent {
        return lookup_in_env(parent, symbol_name);
    }
    None
}

pub fn init_env(env: &mut Env) {
    env.parent = None;
    for i in 0..MAX_BINDINGS {
        env.bindings[i].symbol_name = String::new();
        env.bindings[i].value = None;
    }
}

// =================================================
// Evaluator
// =================================================

fn evaluate_literal_expression(expression: &ExpressionNode, evaluated: &mut Object) {
    if let ExpressionData::Literal(Some(lit)) = &expression.data {
        match lit.type_ {
            LiteralType::Integer => {
                if let LiteralValue::IntValue(n) = &lit.value {
                    evaluated.type_ = ObjectType::Integer;
                    evaluated.value = ObjectValue::IntValue(*n);
                }
            }
            LiteralType::String => {
                if let LiteralValue::StringValue(s) = &lit.value {
                    evaluated.type_ = ObjectType::String;
                    evaluated.value = ObjectValue::StringValue(s.clone());
                }
            }
            LiteralType::Boolean => {
                if let LiteralValue::BooleanValue(b) = &lit.value {
                    evaluated.type_ = ObjectType::Bool;
                    evaluated.value = ObjectValue::BoolValue(if *b { 1 } else { 0 });
                }
            }
        }
    }
}

fn evaluate_symbol_expression(
    expression: &ExpressionNode,
    evaluated: &mut Object,
    env: &mut Env,
) {
    if let ExpressionData::Symbol(Some(sym)) = &expression.data {
        if sym.symbol_name == "nil" {
            evaluated.type_ = ObjectType::Nil;
            evaluated.value = ObjectValue::IntValue(0);
            return;
        }
        if let Some(obj) = lookup_in_env(env, &sym.symbol_name) {
            *evaluated = obj;
            return;
        }
        eprintln!("Undefined symbol: {}", sym.symbol_name);
        std::process::exit(1);
    }
}

fn evaluate_list_expression(
    expression: &ExpressionNode,
    evaluated: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
    let expressions = match &expression.data {
        ExpressionData::List(Some(l)) => l.expressions.as_deref(),
        _ => None,
    };

    // Collect items
    let mut items: Vec<Object> = Vec::new();
    let mut cur = expressions;
    while let Some(node) = cur {
        let mut item = default_object();
        if let Some(expr) = &node.expression {
            evaluate_expression(expr, &mut item, env, context);
        }
        items.push(item);
        cur = node.next.as_deref();
    }

    if items.is_empty() {
        evaluated.type_ = ObjectType::Nil;
        evaluated.value = ObjectValue::IntValue(0);
        return;
    }

    let n = items.len();
    let mut current_cell: Option<ConsCell> = None;
    for (i, item) in items.into_iter().enumerate().rev() {
        let is_last = i == n - 1;
        let cdr_obj = if is_last {
            nil_object()
        } else {
            Object {
                marked: false,
                type_: ObjectType::List,
                value: ObjectValue::ListValue(current_cell.take().map(Box::new)),
            }
        };
        let cell = ConsCell {
            type_: if is_last {
                ConsCellType::Nil
            } else {
                ConsCellType::Cell
            },
            car: Some(Box::new(item)),
            cdr: Some(Box::new(cdr_obj)),
        };
        current_cell = Some(cell);
    }
    evaluated.type_ = ObjectType::List;
    evaluated.value = ObjectValue::ListValue(current_cell.map(Box::new));
}

fn nth_expression<'a>(list: &'a ExpressionList, n: usize) -> Option<&'a ExpressionNode> {
    let mut cur = Some(list);
    let mut i = 0;
    while let Some(node) = cur {
        if i == n {
            return node.expression.as_deref();
        }
        cur = node.next.as_deref();
        i += 1;
    }
    None
}

fn nth_list<'a>(list: &'a ExpressionList, n: usize) -> Option<&'a ExpressionList> {
    let mut cur = Some(list);
    let mut i = 0;
    while let Some(node) = cur {
        if i == n {
            return Some(node);
        }
        cur = node.next.as_deref();
        i += 1;
    }
    None
}

fn evaluate_symbolic_expression(
    expression: &ExpressionNode,
    evaluated: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
    let expressions = match &expression.data {
        ExpressionData::SymbolicExp(Some(s)) => s.expressions.as_deref(),
        _ => None,
    };

    let exprs = match expressions {
        Some(e) => e,
        None => {
            evaluated.type_ = ObjectType::Nil;
            evaluated.value = ObjectValue::IntValue(0);
            return;
        }
    };

    let first_expr = match &exprs.expression {
        Some(e) => e.as_ref(),
        None => {
            evaluated.type_ = ObjectType::Nil;
            evaluated.value = ObjectValue::IntValue(0);
            return;
        }
    };

    let symbol_name = match (first_expr.type_, &first_expr.data) {
        (ExpressionType::Symbol, ExpressionData::Symbol(Some(s))) => s.symbol_name.clone(),
        _ => {
            eprintln!("S-exp must be started with symbol.");
            std::process::exit(1);
        }
    };

    match symbol_name.as_str() {
        "if" => {
            let cond = nth_expression(exprs, 1).expect("if must have condition");
            let then = nth_expression(exprs, 2).expect("if must have then clause");
            let mut cond_obj = default_object();
            evaluate_expression(cond, &mut cond_obj, env, context);
            if bool_val(&cond_obj) {
                evaluate_expression(then, evaluated, env, context);
            } else {
                if let Some(els) = nth_expression(exprs, 3) {
                    evaluate_expression(els, evaluated, env, context);
                } else {
                    evaluated.type_ = ObjectType::Nil;
                    evaluated.value = ObjectValue::IntValue(0);
                }
            }
        }
        "while" => {
            let cond = nth_expression(exprs, 1).expect("while must have condition");
            let then = nth_expression(exprs, 2).expect("while must have body");
            let cond_clone = clone_expression_node(cond);
            let then_clone = clone_expression_node(then);
            loop {
                let mut cond_obj = default_object();
                evaluate_expression(&cond_clone, &mut cond_obj, env, context);
                if bool_val(&cond_obj) {
                    evaluate_expression(&then_clone, evaluated, env, context);
                } else {
                    evaluated.type_ = ObjectType::Nil;
                    evaluated.value = ObjectValue::IntValue(0);
                    break;
                }
            }
        }
        "=" => {
            let var_expr = nth_expression(exprs, 1).expect("= must have variable");
            let var_name = match (&var_expr.type_, &var_expr.data) {
                (ExpressionType::Symbol, ExpressionData::Symbol(Some(s))) => s.symbol_name.clone(),
                _ => {
                    eprintln!("Variable name must be symbol.");
                    std::process::exit(1);
                }
            };
            let val_expr = nth_expression(exprs, 2).expect("= must have value");
            let mut new_obj = default_object();
            evaluate_expression(val_expr, &mut new_obj, env, context);
            *evaluated = clone_object(&new_obj);
            set_object_to_env(env, &var_name, Box::new(new_obj));
        }
        "defun" => {
            let name_expr = nth_expression(exprs, 1).expect("defun must have name");
            let func_name = match (&name_expr.type_, &name_expr.data) {
                (ExpressionType::Symbol, ExpressionData::Symbol(Some(s))) => s.symbol_name.clone(),
                _ => {
                    eprintln!("Function name must be symbol.");
                    std::process::exit(1);
                }
            };
            let params_expr = nth_expression(exprs, 2).expect("defun must have params");
            let params_list = match (&params_expr.type_, &params_expr.data) {
                (ExpressionType::SymbolicExp, ExpressionData::SymbolicExp(Some(s))) => {
                    s.expressions.as_deref()
                }
                _ => {
                    eprintln!("Function parameter must be list.");
                    std::process::exit(1);
                }
            };
            let mut param_names = Vec::new();
            let mut p = params_list;
            while let Some(node) = p {
                if let Some(pe) = &node.expression {
                    match (&pe.type_, &pe.data) {
                        (ExpressionType::Symbol, ExpressionData::Symbol(Some(s))) => {
                            param_names.push(s.symbol_name.clone());
                        }
                        _ => {
                            eprintln!("Function parameter must be symbol.");
                            std::process::exit(1);
                        }
                    }
                }
                p = node.next.as_deref();
            }
            let body_expr = nth_expression(exprs, 3).expect("defun must have body");
            let function = Function {
                param_symbol_names: param_names,
                body: Some(Box::new(clone_expression_node(body_expr))),
            };
            evaluated.type_ = ObjectType::Function;
            evaluated.value = ObjectValue::FunctionValue(Some(Box::new(function)));
            let stored = clone_object(evaluated);
            set_object_to_env(env, &func_name, Box::new(stored));
        }
        "+" => {
            let mut op1 = default_object();
            let mut op2 = default_object();
            evaluate_expression(
                nth_expression(exprs, 1).expect("missing arg"),
                &mut op1,
                env,
                context,
            );
            evaluate_expression(
                nth_expression(exprs, 2).expect("missing arg"),
                &mut op2,
                env,
                context,
            );
            defined_function_add(&op1, &op2, evaluated);
        }
        "-" => {
            let mut op1 = default_object();
            let mut op2 = default_object();
            evaluate_expression(
                nth_expression(exprs, 1).expect("missing arg"),
                &mut op1,
                env,
                context,
            );
            evaluate_expression(
                nth_expression(exprs, 2).expect("missing arg"),
                &mut op2,
                env,
                context,
            );
            defined_function_sub(&op1, &op2, evaluated);
        }
        "*" => {
            let mut op1 = default_object();
            let mut op2 = default_object();
            evaluate_expression(
                nth_expression(exprs, 1).expect("missing arg"),
                &mut op1,
                env,
                context,
            );
            evaluate_expression(
                nth_expression(exprs, 2).expect("missing arg"),
                &mut op2,
                env,
                context,
            );
            defined_function_mul(&op1, &op2, evaluated);
        }
        "/" => {
            let mut op1 = default_object();
            let mut op2 = default_object();
            evaluate_expression(
                nth_expression(exprs, 1).expect("missing arg"),
                &mut op1,
                env,
                context,
            );
            evaluate_expression(
                nth_expression(exprs, 2).expect("missing arg"),
                &mut op2,
                env,
                context,
            );
            defined_function_div(&op1, &op2, evaluated);
        }
        "%" => {
            let mut op1 = default_object();
            let mut op2 = default_object();
            evaluate_expression(
                nth_expression(exprs, 1).expect("missing arg"),
                &mut op1,
                env,
                context,
            );
            evaluate_expression(
                nth_expression(exprs, 2).expect("missing arg"),
                &mut op2,
                env,
                context,
            );
            defined_function_mod(&op1, &op2, evaluated);
        }
        "||" => {
            let mut cur = exprs.next.as_deref();
            evaluated.type_ = ObjectType::Bool;
            evaluated.value = ObjectValue::BoolValue(0);
            while let Some(node) = cur {
                if let Some(e) = &node.expression {
                    let mut op = default_object();
                    evaluate_expression(e, &mut op, env, context);
                    if bool_val(&op) {
                        evaluated.type_ = ObjectType::Bool;
                        evaluated.value = ObjectValue::BoolValue(1);
                        return;
                    }
                }
                cur = node.next.as_deref();
            }
        }
        "&&" => {
            let mut cur = exprs.next.as_deref();
            evaluated.type_ = ObjectType::Bool;
            evaluated.value = ObjectValue::BoolValue(1);
            while let Some(node) = cur {
                if let Some(e) = &node.expression {
                    let mut op = default_object();
                    evaluate_expression(e, &mut op, env, context);
                    if !bool_val(&op) {
                        evaluated.type_ = ObjectType::Bool;
                        evaluated.value = ObjectValue::BoolValue(0);
                        return;
                    }
                }
                cur = node.next.as_deref();
            }
        }
        "<" => {
            let mut op1 = default_object();
            let mut op2 = default_object();
            evaluate_expression(
                nth_expression(exprs, 1).expect("missing arg"),
                &mut op1,
                env,
                context,
            );
            evaluate_expression(
                nth_expression(exprs, 2).expect("missing arg"),
                &mut op2,
                env,
                context,
            );
            defined_function_lt(&op1, &op2, evaluated);
        }
        ">" => {
            let mut op1 = default_object();
            let mut op2 = default_object();
            evaluate_expression(
                nth_expression(exprs, 1).expect("missing arg"),
                &mut op1,
                env,
                context,
            );
            evaluate_expression(
                nth_expression(exprs, 2).expect("missing arg"),
                &mut op2,
                env,
                context,
            );
            defined_function_gt(&op1, &op2, evaluated);
        }
        "eq" => {
            let mut op1 = default_object();
            let mut op2 = default_object();
            evaluate_expression(
                nth_expression(exprs, 1).expect("missing arg"),
                &mut op1,
                env,
                context,
            );
            evaluate_expression(
                nth_expression(exprs, 2).expect("missing arg"),
                &mut op2,
                env,
                context,
            );
            defined_function_eq(&op1, &op2, evaluated);
        }
        "not" => {
            let mut op = default_object();
            evaluate_expression(
                nth_expression(exprs, 1).expect("missing arg"),
                &mut op,
                env,
                context,
            );
            defined_function_not(&op, evaluated);
        }
        "print" => {
            let mut op = default_object();
            evaluate_expression(
                nth_expression(exprs, 1).expect("missing arg"),
                &mut op,
                env,
                context,
            );
            let s = stringify_object(&op);
            println!("{}", s);
            evaluated.type_ = ObjectType::Nil;
            evaluated.value = ObjectValue::IntValue(0);
        }
        "car" => {
            let mut op = default_object();
            evaluate_expression(
                nth_expression(exprs, 1).expect("missing arg"),
                &mut op,
                env,
                context,
            );
            defined_function_car(&op, evaluated);
        }
        "cdr" => {
            let mut op = default_object();
            evaluate_expression(
                nth_expression(exprs, 1).expect("missing arg"),
                &mut op,
                env,
                context,
            );
            defined_function_cdr(&op, evaluated);
        }
        "cons" => {
            let mut op1 = default_object();
            let mut op2 = default_object();
            evaluate_expression(
                nth_expression(exprs, 1).expect("missing arg"),
                &mut op1,
                env,
                context,
            );
            evaluate_expression(
                nth_expression(exprs, 2).expect("missing arg"),
                &mut op2,
                env,
                context,
            );
            defined_function_cons(&op1, &op2, evaluated);
        }
        "readline" => {
            let mut line = String::new();
            match std::io::stdin().read_line(&mut line) {
                Ok(0) => {
                    evaluated.type_ = ObjectType::Nil;
                    evaluated.value = ObjectValue::IntValue(0);
                }
                Ok(_) => {
                    if line.ends_with('\n') {
                        line.pop();
                    }
                    if line.ends_with('\r') {
                        line.pop();
                    }
                    evaluated.type_ = ObjectType::String;
                    evaluated.value = ObjectValue::StringValue(line);
                }
                Err(_) => {
                    evaluated.type_ = ObjectType::Nil;
                    evaluated.value = ObjectValue::IntValue(0);
                }
            }
        }
        "split" => {
            let mut op1 = default_object();
            let mut op2 = default_object();
            evaluate_expression(
                nth_expression(exprs, 1).expect("missing arg"),
                &mut op1,
                env,
                context,
            );
            evaluate_expression(
                nth_expression(exprs, 2).expect("missing arg"),
                &mut op2,
                env,
                context,
            );
            defined_function_split(&op1, &op2, evaluated);
        }
        "list-ref" => {
            let mut op1 = default_object();
            let mut op2 = default_object();
            evaluate_expression(
                nth_expression(exprs, 1).expect("missing arg"),
                &mut op1,
                env,
                context,
            );
            evaluate_expression(
                nth_expression(exprs, 2).expect("missing arg"),
                &mut op2,
                env,
                context,
            );
            defined_function_list_ref(&op1, &op2, evaluated);
        }
        "progn" => {
            let mut cur = exprs.next.as_deref();
            let mut last: Option<Object> = None;
            while let Some(node) = cur {
                if let Some(e) = &node.expression {
                    let mut op = default_object();
                    evaluate_expression(e, &mut op, env, context);
                    last = Some(op);
                }
                cur = node.next.as_deref();
            }
            if let Some(o) = last {
                *evaluated = o;
            } else {
                evaluated.type_ = ObjectType::Nil;
                evaluated.value = ObjectValue::IntValue(0);
            }
        }
        "remove-whitespaces" => {
            let mut op = default_object();
            evaluate_expression(
                nth_expression(exprs, 1).expect("missing arg"),
                &mut op,
                env,
                context,
            );
            defined_function_remove_whitespaces(&op, evaluated);
        }
        "pop" => {
            let mut op = default_object();
            evaluate_expression(
                nth_expression(exprs, 1).expect("missing arg"),
                &mut op,
                env,
                context,
            );
            defined_function_pop(&op, evaluated);
        }
        "push" => {
            // (push list item) - pushes item onto list
            let mut item_obj = default_object();
            evaluate_expression(
                nth_expression(exprs, 2).expect("missing arg"),
                &mut item_obj,
                env,
                context,
            );
            let mut list_obj = default_object();
            evaluate_expression(
                nth_expression(exprs, 1).expect("missing arg"),
                &mut list_obj,
                env,
                context,
            );
            // Append item to list_obj if list, or create new list if nil
            match list_obj.type_ {
                ObjectType::Nil => {
                    *evaluated = clone_object(&item_obj);
                }
                ObjectType::List => {
                    // Walk to end of list and append a new cell
                    if let ObjectValue::ListValue(Some(_)) = &list_obj.value {
                        // Find list var name (simplified - we'd need to look up the binding)
                        // For now, just construct a new list with item appended
                        let mut items: Vec<Object> = Vec::new();
                        let mut current = match &list_obj.value {
                            ObjectValue::ListValue(c) => c.as_deref(),
                            _ => None,
                        };
                        while let Some(cell) = current {
                            if let Some(car) = &cell.car {
                                items.push(clone_object(car));
                            }
                            if is_last_cons_cell(cell) {
                                break;
                            }
                            current = match cell.cdr.as_deref() {
                                Some(o) => match &o.value {
                                    ObjectValue::ListValue(c) => c.as_deref(),
                                    _ => None,
                                },
                                None => None,
                            };
                        }
                        items.push(clone_object(&item_obj));
                        // Build new list
                        let n = items.len();
                        let mut current_cell: Option<ConsCell> = None;
                        for (i, it) in items.into_iter().enumerate().rev() {
                            let is_last = i == n - 1;
                            let cdr_obj = if is_last {
                                nil_object()
                            } else {
                                Object {
                                    marked: false,
                                    type_: ObjectType::List,
                                    value: ObjectValue::ListValue(
                                        current_cell.take().map(Box::new),
                                    ),
                                }
                            };
                            let cell = ConsCell {
                                type_: if is_last {
                                    ConsCellType::Nil
                                } else {
                                    ConsCellType::Cell
                                },
                                car: Some(Box::new(it)),
                                cdr: Some(Box::new(cdr_obj)),
                            };
                            current_cell = Some(cell);
                        }
                        evaluated.type_ = ObjectType::List;
                        evaluated.value =
                            ObjectValue::ListValue(current_cell.map(Box::new));
                    } else {
                        *evaluated = clone_object(&item_obj);
                    }
                }
                _ => {
                    eprintln!("Type error: push second operand must be list.");
                    std::process::exit(1);
                }
            }
        }
        "length" => {
            let mut op = default_object();
            evaluate_expression(
                nth_expression(exprs, 1).expect("missing arg"),
                &mut op,
                env,
                context,
            );
            defined_function_length(&op, evaluated);
        }
        "is-int-string" => {
            let mut op = default_object();
            evaluate_expression(
                nth_expression(exprs, 1).expect("missing arg"),
                &mut op,
                env,
                context,
            );
            defined_function_is_int_string(&op, evaluated);
        }
        "parse-int" => {
            let mut op = default_object();
            evaluate_expression(
                nth_expression(exprs, 1).expect("missing arg"),
                &mut op,
                env,
                context,
            );
            defined_function_parse_int(&op, evaluated);
        }
        "string-ref" => {
            let mut op1 = default_object();
            let mut op2 = default_object();
            evaluate_expression(
                nth_expression(exprs, 1).expect("missing arg"),
                &mut op1,
                env,
                context,
            );
            evaluate_expression(
                nth_expression(exprs, 2).expect("missing arg"),
                &mut op2,
                env,
                context,
            );
            defined_function_string_ref(&op1, &op2, evaluated);
        }
        _ => {
            // User-defined function call
            let func_obj = match lookup_in_env(env, &symbol_name) {
                Some(o) => o,
                None => {
                    eprintln!("Undefined function: {}", symbol_name);
                    std::process::exit(1);
                }
            };
            let function = match func_obj.value {
                ObjectValue::FunctionValue(Some(f)) => f,
                _ => {
                    eprintln!("Undefined function: {}", symbol_name);
                    std::process::exit(1);
                }
            };

            // Evaluate args in current env
            let mut arg_objects: Vec<Object> = Vec::new();
            let mut arg_iter = exprs.next.as_deref();
            for _ in 0..function.param_symbol_names.len() {
                if let Some(arg_node) = arg_iter {
                    if let Some(arg_expr) = &arg_node.expression {
                        let mut arg_obj = default_object();
                        evaluate_expression(arg_expr, &mut arg_obj, env, context);
                        arg_objects.push(arg_obj);
                    }
                    arg_iter = arg_node.next.as_deref();
                }
            }

            // Build new env with parent = clone(env)
            let mut new_env = Env {
                bindings: std::array::from_fn(|_| empty_binding()),
                parent: Some(Box::new(clone_env(env))),
            };
            for (i, name) in function.param_symbol_names.iter().enumerate() {
                if let Some(arg) = arg_objects.get(i) {
                    set_object_to_env(&mut new_env, name, Box::new(clone_object(arg)));
                }
            }

            if let Some(body) = &function.body {
                evaluate_expression(body, evaluated, &mut new_env, context);
            }
        }
    }
    let _ = nth_list; // suppress unused warning if any
}

pub fn evaluate_expression(
    expression: &ExpressionNode,
    result: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
    match expression.type_ {
        ExpressionType::List => evaluate_list_expression(expression, result, env, context),
        ExpressionType::SymbolicExp => {
            evaluate_symbolic_expression(expression, result, env, context)
        }
        ExpressionType::Literal => evaluate_literal_expression(expression, result),
        ExpressionType::Symbol => evaluate_symbol_expression(expression, result, env),
    }
}

pub fn evaluate_expression_with_context(
    expression: &ExpressionNode,
    result: &mut Object,
    env: &mut Env,
) {
    let mut context = init_allocator();
    evaluate_expression(expression, result, env, &mut context);
}

pub fn evaluate(result: &mut ParseResult) {
    let program = match &result.program {
        Some(p) => p,
        None => return,
    };
    let mut env = Env {
        bindings: std::array::from_fn(|_| empty_binding()),
        parent: None,
    };
    init_env(&mut env);
    let mut context = init_allocator();
    let mut cur = program.expressions.as_deref();
    while let Some(node) = cur {
        if let Some(expr) = &node.expression {
            let mut evaluated = default_object();
            evaluate_expression(expr, &mut evaluated, &mut env, &mut context);
        }
        cur = node.next.as_deref();
    }
}
