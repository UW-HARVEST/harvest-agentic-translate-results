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
//   helpers
// =================================================

fn empty_object() -> Object {
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

fn make_env() -> Env {
    Env {
        bindings: std::array::from_fn(|_| empty_binding()),
        parent: None,
    }
}

fn nil_object() -> Object {
    Object {
        marked: false,
        type_: ObjectType::Nil,
        value: ObjectValue::IntValue(0),
    }
}

fn clone_object(obj: &Object) -> Object {
    Object {
        marked: obj.marked,
        type_: obj.type_,
        value: clone_object_value(&obj.value),
    }
}

fn clone_object_value(v: &ObjectValue) -> ObjectValue {
    match v {
        ObjectValue::IntValue(i) => ObjectValue::IntValue(*i),
        ObjectValue::StringValue(s) => ObjectValue::StringValue(s.clone()),
        ObjectValue::BoolValue(b) => ObjectValue::BoolValue(*b),
        ObjectValue::ListValue(cc) => {
            ObjectValue::ListValue(cc.as_ref().map(|c| Box::new(clone_cons_cell(c))))
        }
        ObjectValue::FunctionValue(f) => {
            ObjectValue::FunctionValue(f.as_ref().map(|fnc| Box::new(clone_function(fnc))))
        }
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
        body: f.body.as_ref().map(|e| Box::new(clone_expr_node(e))),
    }
}

fn clone_expr_node(e: &ExpressionNode) -> ExpressionNode {
    ExpressionNode {
        type_: e.type_,
        data: clone_expr_data(&e.data),
    }
}

fn clone_expr_data(d: &ExpressionData) -> ExpressionData {
    match d {
        ExpressionData::SymbolicExp(s) => ExpressionData::SymbolicExp(
            s.as_ref().map(|n| Box::new(clone_symbolic_exp_node(n))),
        ),
        ExpressionData::List(s) => {
            ExpressionData::List(s.as_ref().map(|n| Box::new(clone_list_node(n))))
        }
        ExpressionData::Literal(s) => {
            ExpressionData::Literal(s.as_ref().map(|n| Box::new(clone_literal_node(n))))
        }
        ExpressionData::Symbol(s) => {
            ExpressionData::Symbol(s.as_ref().map(|n| Box::new(clone_symbol_node(n))))
        }
    }
}

fn clone_symbolic_exp_node(n: &SymbolicExpNode) -> SymbolicExpNode {
    SymbolicExpNode {
        expressions: n.expressions.as_ref().map(|l| Box::new(clone_expr_list(l))),
    }
}

fn clone_list_node(n: &ListNode) -> ListNode {
    ListNode {
        expressions: n.expressions.as_ref().map(|l| Box::new(clone_expr_list(l))),
    }
}

fn clone_expr_list(l: &ExpressionList) -> ExpressionList {
    ExpressionList {
        expression: l.expression.as_ref().map(|e| Box::new(clone_expr_node(e))),
        next: l.next.as_ref().map(|n| Box::new(clone_expr_list(n))),
    }
}

fn clone_literal_node(n: &LiteralNode) -> LiteralNode {
    LiteralNode {
        type_: n.type_,
        value: clone_literal_value(&n.value),
    }
}

fn clone_literal_value(v: &LiteralValue) -> LiteralValue {
    match v {
        LiteralValue::IntValue(i) => LiteralValue::IntValue(*i),
        LiteralValue::BooleanValue(b) => LiteralValue::BooleanValue(*b),
        LiteralValue::StringValue(s) => LiteralValue::StringValue(s.clone()),
    }
}

fn clone_symbol_node(n: &SymbolNode) -> SymbolNode {
    SymbolNode {
        symbol_name: n.symbol_name.clone(),
    }
}

fn clone_env(env: &Env) -> Env {
    Env {
        bindings: std::array::from_fn(|i| Binding {
            symbol_name: env.bindings[i].symbol_name.clone(),
            value: env.bindings[i]
                .value
                .as_ref()
                .map(|o| Box::new(clone_object(o))),
        }),
        parent: env.parent.as_ref().map(|p| Box::new(clone_env(p))),
    }
}

// Collect all expressions in an ExpressionList into a Vec
fn collect_expressions(list: Option<&ExpressionList>) -> Vec<&ExpressionNode> {
    let mut result = Vec::new();
    let mut current = list;
    while let Some(l) = current {
        if let Some(e) = &l.expression {
            result.push(e.as_ref());
        }
        current = l.next.as_deref();
    }
    result
}

// =================================================
//   tokenizer
// =================================================

fn is_op(ch: u8) -> bool {
    matches!(
        ch,
        b'+' | b'-' | b'*' | b'/' | b'%' | b'|' | b'&' | b'=' | b'<' | b'>'
    )
}

pub fn match_token(state: &mut ParseState, kind: TokenKind) -> i32 {
    if let Some(t) = &state.token {
        if t.kind == kind {
            1
        } else {
            0
        }
    } else {
        0
    }
}

pub fn next(source: &str, state: &mut ParseState) {
    let bytes = source.as_bytes();

    // Skip whitespaces
    while (state.pos as usize) < bytes.len() {
        let c = bytes[state.pos as usize];
        if c.is_ascii_whitespace() || c == b'\n' {
            state.pos += 1;
        } else {
            break;
        }
    }

    let pos = state.pos as usize;
    let c = if pos < bytes.len() { bytes[pos] } else { 0 };

    let new_token: Token = if c == b'(' {
        state.pos += 1;
        Token {
            kind: TokenKind::LParen,
            next: None,
            val: 0,
            str: "(".to_string(),
        }
    } else if c == b')' {
        state.pos += 1;
        Token {
            kind: TokenKind::RParen,
            next: None,
            val: 0,
            str: ")".to_string(),
        }
    } else if c == b'\'' {
        state.pos += 1;
        Token {
            kind: TokenKind::Quote,
            next: None,
            val: 0,
            str: "'".to_string(),
        }
    } else if c == 0 {
        Token {
            kind: TokenKind::Eof,
            next: None,
            val: 0,
            str: "\0".to_string(),
        }
    } else if c.is_ascii_alphabetic() || is_op(c) {
        let start = pos;
        while (state.pos as usize) < bytes.len() {
            let ch = bytes[state.pos as usize];
            if ch.is_ascii_alphanumeric() || is_op(ch) {
                state.pos += 1;
            } else {
                break;
            }
        }
        let s = std::str::from_utf8(&bytes[start..state.pos as usize])
            .unwrap()
            .to_string();
        match s.as_str() {
            "true" => Token {
                kind: TokenKind::True,
                next: None,
                val: 0,
                str: String::new(),
            },
            "false" => Token {
                kind: TokenKind::False,
                next: None,
                val: 0,
                str: String::new(),
            },
            _ => Token {
                kind: TokenKind::Symbol,
                next: None,
                val: 0,
                str: s,
            },
        }
    } else if c.is_ascii_digit() {
        let start = pos;
        while (state.pos as usize) < bytes.len() && bytes[state.pos as usize].is_ascii_digit() {
            state.pos += 1;
        }
        let s = std::str::from_utf8(&bytes[start..state.pos as usize]).unwrap();
        let val: i32 = s.parse().unwrap_or(0);
        Token {
            kind: TokenKind::Digit,
            next: None,
            val,
            str: String::new(),
        }
    } else if c == b'"' {
        state.pos += 1;
        let start = state.pos as usize;
        while (state.pos as usize) < bytes.len() && bytes[state.pos as usize] != b'"' {
            state.pos += 1;
        }
        let s = std::str::from_utf8(&bytes[start..state.pos as usize])
            .unwrap()
            .to_string();
        if (state.pos as usize) < bytes.len() && bytes[state.pos as usize] == b'"' {
            state.pos += 1;
        }
        Token {
            kind: TokenKind::String,
            next: None,
            val: 0,
            str: s,
        }
    } else if c == b';' {
        while (state.pos as usize) < bytes.len() && bytes[state.pos as usize] != b'\n' {
            state.pos += 1;
        }
        next(source, state);
        return;
    } else {
        panic!("Unexpected token: {}", c as char);
    };

    state.token = Some(Box::new(new_token));
}

// =================================================
//   parser
// =================================================

fn parse_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    if match_token(state, TokenKind::LParen) == 1 {
        parse_symbolic_expression(source, state)
    } else if match_token(state, TokenKind::Quote) == 1 {
        parse_list_expression(source, state)
    } else if match_token(state, TokenKind::Symbol) == 1 {
        parse_symbol_expression(source, state)
    } else if match_token(state, TokenKind::Digit) == 1
        || match_token(state, TokenKind::String) == 1
        || match_token(state, TokenKind::True) == 1
        || match_token(state, TokenKind::False) == 1
    {
        parse_literal_expression(source, state)
    } else {
        let s = state
            .token
            .as_ref()
            .map(|t| t.str.clone())
            .unwrap_or_default();
        panic!("Unexpected token: {}", s);
    }
}

fn parse_symbolic_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    let mut expressions: Option<Box<ExpressionList>> = None;
    next(source, state); // eat '('
    while match_token(state, TokenKind::RParen) != 1 {
        let item = parse_expression(source, state);
        let new_list = Box::new(ExpressionList {
            expression: Some(Box::new(item)),
            next: None,
        });
        // Append to end
        if expressions.is_none() {
            expressions = Some(new_list);
        } else {
            let mut current = expressions.as_deref_mut().unwrap();
            while current.next.is_some() {
                current = current.next.as_deref_mut().unwrap();
            }
            current.next = Some(new_list);
        }
    }
    next(source, state); // eat ')'
    ExpressionNode {
        type_: ExpressionType::SymbolicExp,
        data: ExpressionData::SymbolicExp(Some(Box::new(SymbolicExpNode { expressions }))),
    }
}

fn parse_list_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    let mut expressions: Option<Box<ExpressionList>> = None;
    next(source, state); // eat quote
    next(source, state); // eat '('
    while match_token(state, TokenKind::RParen) != 1 {
        let item = parse_expression(source, state);
        let new_list = Box::new(ExpressionList {
            expression: Some(Box::new(item)),
            next: None,
        });
        if expressions.is_none() {
            expressions = Some(new_list);
        } else {
            let mut current = expressions.as_deref_mut().unwrap();
            while current.next.is_some() {
                current = current.next.as_deref_mut().unwrap();
            }
            current.next = Some(new_list);
        }
    }
    next(source, state); // eat ')'
    ExpressionNode {
        type_: ExpressionType::List,
        data: ExpressionData::List(Some(Box::new(ListNode { expressions }))),
    }
}

fn parse_symbol_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    let symbol_name = state
        .token
        .as_ref()
        .map(|t| t.str.clone())
        .unwrap_or_default();
    let node = ExpressionNode {
        type_: ExpressionType::Symbol,
        data: ExpressionData::Symbol(Some(Box::new(SymbolNode { symbol_name }))),
    };
    next(source, state);
    node
}

fn parse_literal_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    let node = if match_token(state, TokenKind::Digit) == 1 {
        let val = state.token.as_ref().map(|t| t.val).unwrap_or(0);
        ExpressionNode {
            type_: ExpressionType::Literal,
            data: ExpressionData::Literal(Some(Box::new(LiteralNode {
                type_: LiteralType::Integer,
                value: LiteralValue::IntValue(val),
            }))),
        }
    } else if match_token(state, TokenKind::String) == 1 {
        let s = state
            .token
            .as_ref()
            .map(|t| t.str.clone())
            .unwrap_or_default();
        ExpressionNode {
            type_: ExpressionType::Literal,
            data: ExpressionData::Literal(Some(Box::new(LiteralNode {
                type_: LiteralType::String,
                value: LiteralValue::StringValue(s),
            }))),
        }
    } else if match_token(state, TokenKind::True) == 1 {
        ExpressionNode {
            type_: ExpressionType::Literal,
            data: ExpressionData::Literal(Some(Box::new(LiteralNode {
                type_: LiteralType::Boolean,
                value: LiteralValue::BooleanValue(true),
            }))),
        }
    } else if match_token(state, TokenKind::False) == 1 {
        ExpressionNode {
            type_: ExpressionType::Literal,
            data: ExpressionData::Literal(Some(Box::new(LiteralNode {
                type_: LiteralType::Boolean,
                value: LiteralValue::BooleanValue(false),
            }))),
        }
    } else {
        panic!("Unexpected token");
    };
    next(source, state);
    node
}

pub fn parse(source: &str, state: &mut ParseState, result: &mut ParseResult) {
    next(source, state); // set first token

    let mut expressions: Option<Box<ExpressionList>> = None;

    while match_token(state, TokenKind::Eof) != 1 {
        let expr = parse_expression(source, state);
        let new_list = Box::new(ExpressionList {
            expression: Some(Box::new(expr)),
            next: None,
        });
        if expressions.is_none() {
            expressions = Some(new_list);
        } else {
            let mut current = expressions.as_deref_mut().unwrap();
            while current.next.is_some() {
                current = current.next.as_deref_mut().unwrap();
            }
            current.next = Some(new_list);
        }
    }

    result.program = Some(Box::new(ProgramNode { expressions }));
}

// =================================================
//   garbage collector / allocator (simplified)
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
    Some(Box::new(empty_object()))
}

// =================================================
//   evaluator helpers
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

fn eq_objects(op1: &Object, op2: &Object) -> bool {
    match (&op1.type_, &op2.type_) {
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
        (ObjectType::List, ObjectType::List) => {
            // Compare list contents structurally (best-effort)
            list_eq(&op1.value, &op2.value)
        }
        (ObjectType::Nil, ObjectType::Nil) => true,
        _ => false,
    }
}

fn list_eq(a: &ObjectValue, b: &ObjectValue) -> bool {
    match (a, b) {
        (ObjectValue::ListValue(None), ObjectValue::ListValue(None)) => true,
        (ObjectValue::ListValue(Some(x)), ObjectValue::ListValue(Some(y))) => {
            // Structural comparison
            let car_eq = match (&x.car, &y.car) {
                (Some(xc), Some(yc)) => eq_objects(xc, yc),
                (None, None) => true,
                _ => false,
            };
            if !car_eq {
                return false;
            }
            match (&x.cdr, &y.cdr) {
                (Some(xc), Some(yc)) => eq_objects(xc, yc),
                (None, None) => true,
                _ => false,
            }
        }
        _ => false,
    }
}

fn lookup_in_env(env: &Env, name: &str) -> Option<Object> {
    for i in 0..MAX_BINDINGS {
        if !env.bindings[i].symbol_name.is_empty() && env.bindings[i].symbol_name == name {
            if let Some(obj) = &env.bindings[i].value {
                return Some(clone_object(obj));
            }
        }
    }
    if let Some(parent) = &env.parent {
        return lookup_in_env(parent, name);
    }
    None
}

fn set_object_to_env(env: &mut Env, symbol_name: &str, obj: Object) {
    // search binding with the name
    for i in 0..MAX_BINDINGS {
        if !env.bindings[i].symbol_name.is_empty() && env.bindings[i].symbol_name == symbol_name {
            env.bindings[i].value = Some(Box::new(obj));
            return;
        }
    }
    // not found, add new
    for i in 0..MAX_BINDINGS {
        if env.bindings[i].symbol_name.is_empty() {
            env.bindings[i].symbol_name = symbol_name.to_string();
            env.bindings[i].value = Some(Box::new(obj));
            return;
        }
    }
    panic!("Env is full");
}

// =================================================
//   stringify
// =================================================

pub fn stringify_object(obj: &Object) -> String {
    match obj.type_ {
        ObjectType::Integer => match &obj.value {
            ObjectValue::IntValue(i) => i.to_string(),
            _ => String::new(),
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
            _ => String::new(),
        },
        ObjectType::List => {
            let mut out = String::from("(");
            let mut first = true;
            // Walk the cons cells
            if let ObjectValue::ListValue(Some(cell)) = &obj.value {
                let mut cur: Option<&ConsCell> = Some(cell);
                while let Some(c) = cur {
                    if !first {
                        out.push(' ');
                    }
                    first = false;
                    if let Some(car) = &c.car {
                        out.push_str(&stringify_object(car));
                    }
                    // Check if last
                    let is_last = match &c.cdr {
                        Some(cdr) => matches!(cdr.type_, ObjectType::Nil),
                        None => true,
                    };
                    if is_last {
                        break;
                    }
                    // advance to next list cell via cdr's list_value
                    cur = match &c.cdr {
                        Some(cdr) => match &cdr.value {
                            ObjectValue::ListValue(Some(next_cell)) => Some(next_cell.as_ref()),
                            _ => None,
                        },
                        None => None,
                    };
                }
            }
            out.push(')');
            out
        }
        ObjectType::Nil => "nil".to_string(),
        ObjectType::Function => "<function>".to_string(),
    }
}

// =================================================
//   defined functions (built-ins)
// =================================================

fn defined_function_add(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (&op1.type_, &op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => match (&op1.value, &op2.value) {
            (ObjectValue::IntValue(a), ObjectValue::IntValue(b)) => {
                evaluated.type_ = ObjectType::Integer;
                evaluated.value = ObjectValue::IntValue(a + b);
            }
            _ => panic!("Type mismatch"),
        },
        (ObjectType::String, ObjectType::String) => match (&op1.value, &op2.value) {
            (ObjectValue::StringValue(a), ObjectValue::StringValue(b)) => {
                evaluated.type_ = ObjectType::String;
                evaluated.value = ObjectValue::StringValue(format!("{}{}", a, b));
            }
            _ => panic!("Type mismatch"),
        },
        _ => panic!("Type error: operands for + must be integers or strings."),
    }
}

fn defined_function_sub(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (&op1.value, &op2.value) {
        (ObjectValue::IntValue(a), ObjectValue::IntValue(b)) => {
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(a - b);
        }
        _ => panic!("Type error: operands for - must be integers."),
    }
}

fn defined_function_mul(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (&op1.value, &op2.value) {
        (ObjectValue::IntValue(a), ObjectValue::IntValue(b)) => {
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(a * b);
        }
        _ => panic!("Type error: operands for * must be integers."),
    }
}

fn defined_function_div(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (&op1.value, &op2.value) {
        (ObjectValue::IntValue(a), ObjectValue::IntValue(b)) => {
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(a / b);
        }
        _ => panic!("Type error: operands for / must be integers."),
    }
}

fn defined_function_mod(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (&op1.value, &op2.value) {
        (ObjectValue::IntValue(a), ObjectValue::IntValue(b)) => {
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(a % b);
        }
        _ => panic!("Type error: operands for % must be integers."),
    }
}

fn defined_function_lt(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (&op1.value, &op2.value) {
        (ObjectValue::IntValue(a), ObjectValue::IntValue(b)) => {
            evaluated.type_ = ObjectType::Bool;
            evaluated.value = ObjectValue::BoolValue(if a < b { 1 } else { 0 });
        }
        _ => panic!("Type error: operands for < must be integers."),
    }
}

fn defined_function_gt(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (&op1.value, &op2.value) {
        (ObjectValue::IntValue(a), ObjectValue::IntValue(b)) => {
            evaluated.type_ = ObjectType::Bool;
            evaluated.value = ObjectValue::BoolValue(if a > b { 1 } else { 0 });
        }
        _ => panic!("Type error: operands for > must be integers."),
    }
}

fn defined_function_eq(op1: &Object, op2: &Object, evaluated: &mut Object) {
    let eq = eq_objects(op1, op2);
    evaluated.type_ = ObjectType::Bool;
    evaluated.value = ObjectValue::BoolValue(if eq { 1 } else { 0 });
}

fn defined_function_not(op: &Object, evaluated: &mut Object) {
    if !matches!(op.type_, ObjectType::Bool) {
        panic!("Type error: not operand must be boolean.");
    }
    evaluated.type_ = ObjectType::Bool;
    let v = match &op.value {
        ObjectValue::BoolValue(b) => *b,
        _ => 0,
    };
    evaluated.value = ObjectValue::BoolValue(if v != 0 { 0 } else { 1 });
}

fn defined_function_car(op: &Object, evaluated: &mut Object) {
    if !matches!(op.type_, ObjectType::List) {
        panic!("Type error: car operand must be list.");
    }
    if let ObjectValue::ListValue(Some(cell)) = &op.value {
        if let Some(car) = &cell.car {
            *evaluated = clone_object(car);
        } else {
            evaluated.type_ = ObjectType::Nil;
            evaluated.value = ObjectValue::IntValue(0);
        }
    } else {
        evaluated.type_ = ObjectType::Nil;
        evaluated.value = ObjectValue::IntValue(0);
    }
}

fn defined_function_cdr(op: &Object, evaluated: &mut Object) {
    if !matches!(op.type_, ObjectType::List) {
        panic!("Type error: cdr operand must be list.");
    }
    if let ObjectValue::ListValue(Some(cell)) = &op.value {
        if let Some(cdr) = &cell.cdr {
            *evaluated = clone_object(cdr);
        } else {
            evaluated.type_ = ObjectType::Nil;
            evaluated.value = ObjectValue::IntValue(0);
        }
    } else {
        evaluated.type_ = ObjectType::Nil;
        evaluated.value = ObjectValue::IntValue(0);
    }
}

fn defined_function_cons(op1: &Object, op2: &Object, evaluated: &mut Object) {
    let car = Box::new(clone_object(op1));
    let (cdr_obj, cell_type) = match &op2.type_ {
        ObjectType::List => (Box::new(clone_object(op2)), ConsCellType::Cell),
        ObjectType::Nil => (Box::new(clone_object(op2)), ConsCellType::Nil),
        _ => {
            // Wrap op2 in a list ending with nil
            let inner_cell = ConsCell {
                type_: ConsCellType::Nil,
                car: Some(Box::new(clone_object(op2))),
                cdr: Some(Box::new(nil_object())),
            };
            let cdr_list = Object {
                marked: false,
                type_: ObjectType::List,
                value: ObjectValue::ListValue(Some(Box::new(inner_cell))),
            };
            (Box::new(cdr_list), ConsCellType::Cell)
        }
    };

    let cell = ConsCell {
        type_: cell_type,
        car: Some(car),
        cdr: Some(cdr_obj),
    };

    evaluated.type_ = ObjectType::List;
    evaluated.value = ObjectValue::ListValue(Some(Box::new(cell)));
}

fn defined_function_split(op1: &Object, op2: &Object, evaluated: &mut Object) {
    let s1 = match (&op1.type_, &op1.value) {
        (ObjectType::String, ObjectValue::StringValue(s)) => s.clone(),
        _ => panic!("Type error: split first operand must be string."),
    };
    let s2 = match (&op2.type_, &op2.value) {
        (ObjectType::String, ObjectValue::StringValue(s)) => s.clone(),
        _ => panic!("Type error: split second operand must be string."),
    };

    let parts: Vec<String> = if s2.is_empty() {
        s1.chars().map(|c| c.to_string()).collect()
    } else {
        s1.split(|c: char| s2.contains(c))
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    };

    if parts.is_empty() {
        evaluated.type_ = ObjectType::Nil;
        evaluated.value = ObjectValue::IntValue(0);
        return;
    }

    // Build cons cell list
    let mut iter = parts.into_iter().rev();
    let last = iter.next().unwrap();
    let last_obj = Object {
        marked: false,
        type_: ObjectType::String,
        value: ObjectValue::StringValue(last),
    };
    let mut current_cell = ConsCell {
        type_: ConsCellType::Nil,
        car: Some(Box::new(last_obj)),
        cdr: Some(Box::new(nil_object())),
    };
    for s in iter {
        let car_obj = Object {
            marked: false,
            type_: ObjectType::String,
            value: ObjectValue::StringValue(s),
        };
        let cdr_obj = Object {
            marked: false,
            type_: ObjectType::List,
            value: ObjectValue::ListValue(Some(Box::new(current_cell))),
        };
        current_cell = ConsCell {
            type_: ConsCellType::Cell,
            car: Some(Box::new(car_obj)),
            cdr: Some(Box::new(cdr_obj)),
        };
    }

    evaluated.type_ = ObjectType::List;
    evaluated.value = ObjectValue::ListValue(Some(Box::new(current_cell)));
}

fn defined_function_list_ref(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if !matches!(op1.type_, ObjectType::List) {
        panic!("Type error: list-ref first operand must be list.");
    }
    let index = match (&op2.type_, &op2.value) {
        (ObjectType::Integer, ObjectValue::IntValue(i)) => *i,
        _ => panic!("Type error: list-ref second operand must be integer."),
    };

    let mut current_cell: Option<&ConsCell> = match &op1.value {
        ObjectValue::ListValue(Some(c)) => Some(c.as_ref()),
        _ => None,
    };

    for _ in 0..index {
        match current_cell {
            Some(c) => {
                // Move to next list cell via cdr
                if let Some(cdr) = &c.cdr {
                    if matches!(cdr.type_, ObjectType::Nil) {
                        panic!("Index out of range.");
                    }
                    current_cell = match &cdr.value {
                        ObjectValue::ListValue(Some(next_c)) => Some(next_c.as_ref()),
                        _ => panic!("Index out of range."),
                    };
                } else {
                    panic!("Index out of range.");
                }
            }
            None => panic!("Index out of range."),
        }
    }

    if let Some(c) = current_cell {
        if let Some(car) = &c.car {
            *evaluated = clone_object(car);
        } else {
            evaluated.type_ = ObjectType::Nil;
            evaluated.value = ObjectValue::IntValue(0);
        }
    }
}

fn defined_function_remove_whitespaces(op: &Object, evaluated: &mut Object) {
    let s = match (&op.type_, &op.value) {
        (ObjectType::String, ObjectValue::StringValue(s)) => s.clone(),
        _ => panic!("Type error: remove-whitespaces operand must be string."),
    };
    let new_s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    evaluated.type_ = ObjectType::String;
    evaluated.value = ObjectValue::StringValue(new_s);
}

fn defined_function_pop(op: &Object, evaluated: &mut Object) {
    if matches!(op.type_, ObjectType::Nil) {
        evaluated.type_ = ObjectType::Nil;
        evaluated.value = ObjectValue::IntValue(0);
        return;
    }
    if !matches!(op.type_, ObjectType::List) {
        panic!("Type error: pop operand must be list.");
    }

    // Find last car element
    let mut current: Option<&ConsCell> = match &op.value {
        ObjectValue::ListValue(Some(c)) => Some(c.as_ref()),
        _ => None,
    };

    while let Some(c) = current {
        let is_last = match &c.cdr {
            Some(cdr) => matches!(cdr.type_, ObjectType::Nil),
            None => true,
        };
        if is_last {
            if let Some(car) = &c.car {
                *evaluated = clone_object(car);
            } else {
                evaluated.type_ = ObjectType::Nil;
                evaluated.value = ObjectValue::IntValue(0);
            }
            return;
        }
        current = match &c.cdr {
            Some(cdr) => match &cdr.value {
                ObjectValue::ListValue(Some(next_c)) => Some(next_c.as_ref()),
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
            let mut current: Option<&ConsCell> = match &op.value {
                ObjectValue::ListValue(Some(c)) => Some(c.as_ref()),
                _ => None,
            };
            while let Some(c) = current {
                len += 1;
                let is_last = match &c.cdr {
                    Some(cdr) => matches!(cdr.type_, ObjectType::Nil),
                    None => true,
                };
                if is_last {
                    break;
                }
                current = match &c.cdr {
                    Some(cdr) => match &cdr.value {
                        ObjectValue::ListValue(Some(next_c)) => Some(next_c.as_ref()),
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
        _ => panic!("Type error: length operand must be list or string."),
    }
}

fn defined_function_is_int_string(op: &Object, evaluated: &mut Object) {
    match (&op.type_, &op.value) {
        (ObjectType::String, ObjectValue::StringValue(s)) => {
            let is_digits = !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
            evaluated.type_ = ObjectType::Bool;
            evaluated.value = ObjectValue::BoolValue(if is_digits { 1 } else { 0 });
        }
        _ => {
            evaluated.type_ = ObjectType::Bool;
            evaluated.value = ObjectValue::BoolValue(0);
        }
    }
}

fn defined_function_parse_int(op: &Object, evaluated: &mut Object) {
    let s = match (&op.type_, &op.value) {
        (ObjectType::String, ObjectValue::StringValue(s)) => s.clone(),
        _ => panic!("Type error: parse-int operand must be string."),
    };
    if s.is_empty() || !s.chars().all(|c| c.is_ascii_digit()) {
        panic!("Type error: parse-int operand must be string of digits.");
    }
    let v: i32 = s.parse().unwrap_or(0);
    evaluated.type_ = ObjectType::Integer;
    evaluated.value = ObjectValue::IntValue(v);
}

fn defined_function_string_ref(op1: &Object, op2: &Object, evaluated: &mut Object) {
    let s = match (&op1.type_, &op1.value) {
        (ObjectType::String, ObjectValue::StringValue(s)) => s.clone(),
        _ => panic!("Type error: string-ref first operand must be string."),
    };
    let idx = match (&op2.type_, &op2.value) {
        (ObjectType::Integer, ObjectValue::IntValue(i)) => *i,
        _ => panic!("Type error: string-ref second operand must be integer."),
    };
    let bytes = s.as_bytes();
    if idx < 0 || (idx as usize) >= bytes.len() {
        panic!("Index out of range.");
    }
    let ch = bytes[idx as usize] as char;
    evaluated.type_ = ObjectType::String;
    evaluated.value = ObjectValue::StringValue(ch.to_string());
}

// =================================================
//   evaluator
// =================================================

fn evaluate_list_expression_inner(
    expression: &ExpressionNode,
    evaluated: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
    let expressions_opt = match &expression.data {
        ExpressionData::List(Some(node)) => node.expressions.as_deref(),
        _ => return,
    };

    if expressions_opt.is_none() {
        evaluated.type_ = ObjectType::Nil;
        evaluated.value = ObjectValue::IntValue(0);
        return;
    }

    // Evaluate all items in order (so side effects on env propagate)
    let mut items: Vec<Object> = Vec::new();
    let mut current = expressions_opt;
    while let Some(list) = current {
        if let Some(expr) = &list.expression {
            let cloned_expr = clone_expr_node(expr);
            let mut item = empty_object();
            evaluate_expression(&cloned_expr, &mut item, env, context);
            items.push(item);
        }
        current = list.next.as_deref();
    }

    if items.is_empty() {
        evaluated.type_ = ObjectType::Nil;
        evaluated.value = ObjectValue::IntValue(0);
        return;
    }

    // Build cons cells from end to start
    let mut iter = items.into_iter().rev();
    let last_car = iter.next().unwrap();
    let mut current_cell = ConsCell {
        type_: ConsCellType::Nil,
        car: Some(Box::new(last_car)),
        cdr: Some(Box::new(nil_object())),
    };
    for car in iter {
        let cdr_obj = Object {
            marked: false,
            type_: ObjectType::List,
            value: ObjectValue::ListValue(Some(Box::new(current_cell))),
        };
        current_cell = ConsCell {
            type_: ConsCellType::Cell,
            car: Some(Box::new(car)),
            cdr: Some(Box::new(cdr_obj)),
        };
    }

    evaluated.type_ = ObjectType::List;
    evaluated.value = ObjectValue::ListValue(Some(Box::new(current_cell)));
}

fn get_symbol_name(expr: &ExpressionNode) -> Option<String> {
    match &expr.data {
        ExpressionData::Symbol(Some(s)) => Some(s.symbol_name.clone()),
        _ => None,
    }
}

fn evaluate_symbolic_expression_inner(
    expression: &ExpressionNode,
    evaluated: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
    let expressions_opt = match &expression.data {
        ExpressionData::SymbolicExp(Some(node)) => node.expressions.as_deref(),
        _ => {
            evaluated.type_ = ObjectType::Nil;
            evaluated.value = ObjectValue::IntValue(0);
            return;
        }
    };

    let exprs = match expressions_opt {
        Some(e) => e,
        None => {
            evaluated.type_ = ObjectType::Nil;
            evaluated.value = ObjectValue::IntValue(0);
            return;
        }
    };

    // The first expression must be a symbol
    let head_expr = match &exprs.expression {
        Some(e) => e.as_ref(),
        None => {
            evaluated.type_ = ObjectType::Nil;
            evaluated.value = ObjectValue::IntValue(0);
            return;
        }
    };

    let symbol_name = match get_symbol_name(head_expr) {
        Some(s) => s,
        None => panic!("S-exp must be started with symbol."),
    };

    // Collect arg expressions (cloned)
    let arg_exprs: Vec<ExpressionNode> = collect_expressions(exprs.next.as_deref())
        .into_iter()
        .map(clone_expr_node)
        .collect();

    match symbol_name.as_str() {
        "if" => {
            if arg_exprs.is_empty() {
                panic!("if must have condition.");
            }
            if arg_exprs.len() < 2 {
                panic!("if must have then clause.");
            }
            let cond_expr = &arg_exprs[0];
            let then_expr = &arg_exprs[1];
            let mut cond_obj = empty_object();
            evaluate_expression(cond_expr, &mut cond_obj, env, context);
            if bool_val(&cond_obj) {
                evaluate_expression(then_expr, evaluated, env, context);
            } else if arg_exprs.len() >= 3 {
                evaluate_expression(&arg_exprs[2], evaluated, env, context);
            } else {
                evaluated.type_ = ObjectType::Nil;
                evaluated.value = ObjectValue::IntValue(0);
            }
        }
        "while" => {
            if arg_exprs.is_empty() {
                panic!("while must have condition.");
            }
            if arg_exprs.len() < 2 {
                panic!("while must have body.");
            }
            let cond_expr = &arg_exprs[0];
            let body_expr = &arg_exprs[1];
            loop {
                let mut cond_obj = empty_object();
                evaluate_expression(cond_expr, &mut cond_obj, env, context);
                if bool_val(&cond_obj) {
                    evaluate_expression(body_expr, evaluated, env, context);
                } else {
                    evaluated.type_ = ObjectType::Nil;
                    evaluated.value = ObjectValue::IntValue(0);
                    break;
                }
            }
        }
        "=" => {
            if arg_exprs.is_empty() {
                panic!("assignment must have variable name.");
            }
            let var_name = match get_symbol_name(&arg_exprs[0]) {
                Some(s) => s,
                None => panic!("Variable name must be symbol."),
            };
            if arg_exprs.len() < 2 {
                panic!("assignment must have expression.");
            }
            let mut val = empty_object();
            evaluate_expression(&arg_exprs[1], &mut val, env, context);
            let val_clone = clone_object(&val);
            *evaluated = val;
            set_object_to_env(env, &var_name, val_clone);
        }
        "defun" => {
            if arg_exprs.is_empty() {
                panic!("Function name required.");
            }
            let fn_name = match get_symbol_name(&arg_exprs[0]) {
                Some(s) => s,
                None => panic!("Function name must be symbol."),
            };
            if arg_exprs.len() < 2 {
                panic!("Function must have parameters.");
            }
            let params_expr = &arg_exprs[1];
            if !matches!(params_expr.type_, ExpressionType::SymbolicExp) {
                panic!("Function parameter must be list.");
            }
            let param_names: Vec<String> = match &params_expr.data {
                ExpressionData::SymbolicExp(Some(node)) => {
                    let plist = collect_expressions(node.expressions.as_deref());
                    plist
                        .into_iter()
                        .map(|p| match get_symbol_name(p) {
                            Some(s) => s,
                            None => panic!("Function parameter must be symbol."),
                        })
                        .collect()
                }
                _ => Vec::new(),
            };
            if arg_exprs.len() < 3 {
                panic!("Function must have body.");
            }
            let body = clone_expr_node(&arg_exprs[2]);
            let function = Function {
                param_symbol_names: param_names,
                body: Some(Box::new(body)),
            };
            let func_obj = Object {
                marked: false,
                type_: ObjectType::Function,
                value: ObjectValue::FunctionValue(Some(Box::new(function))),
            };
            let env_obj = clone_object(&func_obj);
            *evaluated = func_obj;
            set_object_to_env(env, &fn_name, env_obj);
        }
        "+" => {
            let mut o1 = empty_object();
            let mut o2 = empty_object();
            evaluate_expression(&arg_exprs[0], &mut o1, env, context);
            evaluate_expression(&arg_exprs[1], &mut o2, env, context);
            defined_function_add(&o1, &o2, evaluated);
        }
        "-" => {
            let mut o1 = empty_object();
            let mut o2 = empty_object();
            evaluate_expression(&arg_exprs[0], &mut o1, env, context);
            evaluate_expression(&arg_exprs[1], &mut o2, env, context);
            defined_function_sub(&o1, &o2, evaluated);
        }
        "*" => {
            let mut o1 = empty_object();
            let mut o2 = empty_object();
            evaluate_expression(&arg_exprs[0], &mut o1, env, context);
            evaluate_expression(&arg_exprs[1], &mut o2, env, context);
            defined_function_mul(&o1, &o2, evaluated);
        }
        "/" => {
            let mut o1 = empty_object();
            let mut o2 = empty_object();
            evaluate_expression(&arg_exprs[0], &mut o1, env, context);
            evaluate_expression(&arg_exprs[1], &mut o2, env, context);
            defined_function_div(&o1, &o2, evaluated);
        }
        "%" => {
            let mut o1 = empty_object();
            let mut o2 = empty_object();
            evaluate_expression(&arg_exprs[0], &mut o1, env, context);
            evaluate_expression(&arg_exprs[1], &mut o2, env, context);
            defined_function_mod(&o1, &o2, evaluated);
        }
        "||" => {
            for arg in &arg_exprs {
                let mut o = empty_object();
                evaluate_expression(arg, &mut o, env, context);
                if bool_val(&o) {
                    evaluated.type_ = ObjectType::Bool;
                    evaluated.value = ObjectValue::BoolValue(1);
                    return;
                }
            }
            evaluated.type_ = ObjectType::Bool;
            evaluated.value = ObjectValue::BoolValue(0);
        }
        "&&" => {
            for arg in &arg_exprs {
                let mut o = empty_object();
                evaluate_expression(arg, &mut o, env, context);
                if !bool_val(&o) {
                    evaluated.type_ = ObjectType::Bool;
                    evaluated.value = ObjectValue::BoolValue(0);
                    return;
                }
            }
            evaluated.type_ = ObjectType::Bool;
            evaluated.value = ObjectValue::BoolValue(1);
        }
        "<" => {
            let mut o1 = empty_object();
            let mut o2 = empty_object();
            evaluate_expression(&arg_exprs[0], &mut o1, env, context);
            evaluate_expression(&arg_exprs[1], &mut o2, env, context);
            defined_function_lt(&o1, &o2, evaluated);
        }
        ">" => {
            let mut o1 = empty_object();
            let mut o2 = empty_object();
            evaluate_expression(&arg_exprs[0], &mut o1, env, context);
            evaluate_expression(&arg_exprs[1], &mut o2, env, context);
            defined_function_gt(&o1, &o2, evaluated);
        }
        "eq" => {
            let mut o1 = empty_object();
            let mut o2 = empty_object();
            evaluate_expression(&arg_exprs[0], &mut o1, env, context);
            evaluate_expression(&arg_exprs[1], &mut o2, env, context);
            defined_function_eq(&o1, &o2, evaluated);
        }
        "not" => {
            let mut o = empty_object();
            evaluate_expression(&arg_exprs[0], &mut o, env, context);
            defined_function_not(&o, evaluated);
        }
        "print" => {
            let mut o = empty_object();
            evaluate_expression(&arg_exprs[0], &mut o, env, context);
            let s = stringify_object(&o);
            println!("{}", s);
            evaluated.type_ = ObjectType::Nil;
            evaluated.value = ObjectValue::IntValue(0);
        }
        "car" => {
            let mut o = empty_object();
            evaluate_expression(&arg_exprs[0], &mut o, env, context);
            defined_function_car(&o, evaluated);
        }
        "cdr" => {
            let mut o = empty_object();
            evaluate_expression(&arg_exprs[0], &mut o, env, context);
            defined_function_cdr(&o, evaluated);
        }
        "cons" => {
            let mut o1 = empty_object();
            let mut o2 = empty_object();
            evaluate_expression(&arg_exprs[0], &mut o1, env, context);
            evaluate_expression(&arg_exprs[1], &mut o2, env, context);
            defined_function_cons(&o1, &o2, evaluated);
        }
        "readline" => {
            use std::io::BufRead;
            let stdin = std::io::stdin();
            let mut line = String::new();
            match stdin.lock().read_line(&mut line) {
                Ok(0) => {
                    evaluated.type_ = ObjectType::Nil;
                    evaluated.value = ObjectValue::IntValue(0);
                }
                Ok(_) => {
                    if line.ends_with('\n') {
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
            let mut o1 = empty_object();
            let mut o2 = empty_object();
            evaluate_expression(&arg_exprs[0], &mut o1, env, context);
            evaluate_expression(&arg_exprs[1], &mut o2, env, context);
            defined_function_split(&o1, &o2, evaluated);
        }
        "list-ref" => {
            let mut o1 = empty_object();
            let mut o2 = empty_object();
            evaluate_expression(&arg_exprs[0], &mut o1, env, context);
            evaluate_expression(&arg_exprs[1], &mut o2, env, context);
            defined_function_list_ref(&o1, &o2, evaluated);
        }
        "progn" => {
            let mut last: Option<Object> = None;
            for arg in &arg_exprs {
                let mut o = empty_object();
                evaluate_expression(arg, &mut o, env, context);
                last = Some(o);
            }
            if let Some(o) = last {
                *evaluated = o;
            } else {
                evaluated.type_ = ObjectType::Nil;
                evaluated.value = ObjectValue::IntValue(0);
            }
        }
        "remove-whitespaces" => {
            let mut o = empty_object();
            evaluate_expression(&arg_exprs[0], &mut o, env, context);
            defined_function_remove_whitespaces(&o, evaluated);
        }
        "pop" => {
            let mut o = empty_object();
            evaluate_expression(&arg_exprs[0], &mut o, env, context);
            defined_function_pop(&o, evaluated);
        }
        "push" => {
            // push: (push list value) per the wire-up: arg[0] is list expr, arg[1] is value expr
            // C code passes (next.next) as op1 and (next) as op2 to definedFunctionPush(op2, op1, ...)
            // i.e. operand1 is the value, operand2 is the list
            let mut value_obj = empty_object();
            let mut list_obj = empty_object();
            evaluate_expression(&arg_exprs[1], &mut value_obj, env, context);
            evaluate_expression(&arg_exprs[0], &mut list_obj, env, context);
            // Append value to list
            let new_list = push_to_list(&list_obj, &value_obj);
            // Update binding if first arg is a symbol
            if let Some(sym) = get_symbol_name(&arg_exprs[0]) {
                set_object_to_env(env, &sym, clone_object(&new_list));
            }
            *evaluated = new_list;
        }
        "length" => {
            let mut o = empty_object();
            evaluate_expression(&arg_exprs[0], &mut o, env, context);
            defined_function_length(&o, evaluated);
        }
        "is-int-string" => {
            let mut o = empty_object();
            evaluate_expression(&arg_exprs[0], &mut o, env, context);
            defined_function_is_int_string(&o, evaluated);
        }
        "parse-int" => {
            let mut o = empty_object();
            evaluate_expression(&arg_exprs[0], &mut o, env, context);
            defined_function_parse_int(&o, evaluated);
        }
        "string-ref" => {
            let mut o1 = empty_object();
            let mut o2 = empty_object();
            evaluate_expression(&arg_exprs[0], &mut o1, env, context);
            evaluate_expression(&arg_exprs[1], &mut o2, env, context);
            defined_function_string_ref(&o1, &o2, evaluated);
        }
        _ => {
            // user-defined function call
            let fn_obj = match lookup_in_env(env, &symbol_name) {
                Some(o) => o,
                None => panic!("Undefined function: {}", symbol_name),
            };
            let function = match fn_obj.value {
                ObjectValue::FunctionValue(Some(f)) => f,
                _ => panic!("{} is not a function", symbol_name),
            };
            // Evaluate arguments in current env
            let mut new_env = make_env();
            new_env.parent = Some(Box::new(clone_env(env)));
            for (i, pname) in function.param_symbol_names.iter().enumerate() {
                if i >= arg_exprs.len() {
                    break;
                }
                let mut param_obj = empty_object();
                evaluate_expression(&arg_exprs[i], &mut param_obj, env, context);
                set_object_to_env(&mut new_env, pname, param_obj);
            }
            if let Some(body) = &function.body {
                evaluate_expression(body, evaluated, &mut new_env, context);
            }
        }
    }
}

fn push_to_list(list: &Object, value: &Object) -> Object {
    // Build a new list with value appended
    if matches!(list.type_, ObjectType::Nil) {
        // singleton list
        let cell = ConsCell {
            type_: ConsCellType::Nil,
            car: Some(Box::new(clone_object(value))),
            cdr: Some(Box::new(nil_object())),
        };
        return Object {
            marked: false,
            type_: ObjectType::List,
            value: ObjectValue::ListValue(Some(Box::new(cell))),
        };
    }

    if !matches!(list.type_, ObjectType::List) {
        panic!("Type error: push second operand must be list.");
    }

    // Collect existing elements
    let mut elems: Vec<Object> = Vec::new();
    let mut cur: Option<&ConsCell> = match &list.value {
        ObjectValue::ListValue(Some(c)) => Some(c.as_ref()),
        _ => None,
    };
    while let Some(c) = cur {
        if let Some(car) = &c.car {
            elems.push(clone_object(car));
        }
        let is_last = match &c.cdr {
            Some(cdr) => matches!(cdr.type_, ObjectType::Nil),
            None => true,
        };
        if is_last {
            break;
        }
        cur = match &c.cdr {
            Some(cdr) => match &cdr.value {
                ObjectValue::ListValue(Some(next_c)) => Some(next_c.as_ref()),
                _ => None,
            },
            None => None,
        };
    }
    elems.push(clone_object(value));

    // build cons cells from end
    let mut iter = elems.into_iter().rev();
    let last = iter.next().unwrap();
    let mut current_cell = ConsCell {
        type_: ConsCellType::Nil,
        car: Some(Box::new(last)),
        cdr: Some(Box::new(nil_object())),
    };
    for e in iter {
        let cdr_obj = Object {
            marked: false,
            type_: ObjectType::List,
            value: ObjectValue::ListValue(Some(Box::new(current_cell))),
        };
        current_cell = ConsCell {
            type_: ConsCellType::Cell,
            car: Some(Box::new(e)),
            cdr: Some(Box::new(cdr_obj)),
        };
    }
    Object {
        marked: false,
        type_: ObjectType::List,
        value: ObjectValue::ListValue(Some(Box::new(current_cell))),
    }
}

fn evaluate_literal_expression_inner(expression: &ExpressionNode, evaluated: &mut Object) {
    if let ExpressionData::Literal(Some(lit)) = &expression.data {
        match lit.type_ {
            LiteralType::Integer => {
                let v = match &lit.value {
                    LiteralValue::IntValue(i) => *i,
                    _ => 0,
                };
                evaluated.type_ = ObjectType::Integer;
                evaluated.value = ObjectValue::IntValue(v);
            }
            LiteralType::String => {
                let s = match &lit.value {
                    LiteralValue::StringValue(s) => s.clone(),
                    _ => String::new(),
                };
                evaluated.type_ = ObjectType::String;
                evaluated.value = ObjectValue::StringValue(s);
            }
            LiteralType::Boolean => {
                let b = match &lit.value {
                    LiteralValue::BooleanValue(b) => *b,
                    _ => false,
                };
                evaluated.type_ = ObjectType::Bool;
                evaluated.value = ObjectValue::BoolValue(if b { 1 } else { 0 });
            }
        }
    }
}

fn evaluate_symbol_expression_inner(
    expression: &ExpressionNode,
    evaluated: &mut Object,
    env: &mut Env,
) {
    let name = match &expression.data {
        ExpressionData::Symbol(Some(s)) => s.symbol_name.clone(),
        _ => return,
    };
    if name == "nil" {
        evaluated.type_ = ObjectType::Nil;
        evaluated.value = ObjectValue::IntValue(0);
        return;
    }
    match lookup_in_env(env, &name) {
        Some(o) => *evaluated = o,
        None => panic!("Undefined symbol: {}", name),
    }
}

pub fn evaluate_expression(
    expression: &ExpressionNode,
    result: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
    match expression.type_ {
        ExpressionType::List => evaluate_list_expression_inner(expression, result, env, context),
        ExpressionType::SymbolicExp => {
            evaluate_symbolic_expression_inner(expression, result, env, context)
        }
        ExpressionType::Literal => evaluate_literal_expression_inner(expression, result),
        ExpressionType::Symbol => evaluate_symbol_expression_inner(expression, result, env),
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
        Some(p) => clone_program(p),
        None => return,
    };

    let mut env = make_env();
    let mut context = init_allocator();

    let mut current = program.expressions.as_deref();
    while let Some(list) = current {
        if let Some(expr) = &list.expression {
            let mut evaluated = empty_object();
            evaluate_expression(expr, &mut evaluated, &mut env, &mut context);
        }
        current = list.next.as_deref();
    }
}

fn clone_program(p: &ProgramNode) -> ProgramNode {
    ProgramNode {
        expressions: p.expressions.as_ref().map(|l| Box::new(clone_expr_list(l))),
    }
}

pub fn init_env(env: &mut Env) {
    for i in 0..MAX_BINDINGS {
        env.bindings[i] = empty_binding();
    }
    env.parent = None;
}
