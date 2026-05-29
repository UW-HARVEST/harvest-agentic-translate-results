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
//   Clone implementations
// =================================================

impl Clone for Token {
    fn clone(&self) -> Self {
        Token {
            kind: self.kind,
            next: self.next.clone(),
            val: self.val,
            str: self.str.clone(),
        }
    }
}

impl Clone for ExpressionList {
    fn clone(&self) -> Self {
        ExpressionList {
            expression: self.expression.clone(),
            next: self.next.clone(),
        }
    }
}

impl Clone for ExpressionNode {
    fn clone(&self) -> Self {
        ExpressionNode {
            type_: self.type_,
            data: self.data.clone(),
        }
    }
}

impl Clone for ExpressionData {
    fn clone(&self) -> Self {
        match self {
            ExpressionData::SymbolicExp(inner) => ExpressionData::SymbolicExp(inner.clone()),
            ExpressionData::List(inner) => ExpressionData::List(inner.clone()),
            ExpressionData::Literal(inner) => ExpressionData::Literal(inner.clone()),
            ExpressionData::Symbol(inner) => ExpressionData::Symbol(inner.clone()),
        }
    }
}

impl Clone for SymbolicExpNode {
    fn clone(&self) -> Self {
        SymbolicExpNode { expressions: self.expressions.clone() }
    }
}

impl Clone for ListNode {
    fn clone(&self) -> Self {
        ListNode { expressions: self.expressions.clone() }
    }
}

impl Clone for LiteralNode {
    fn clone(&self) -> Self {
        LiteralNode {
            type_: self.type_,
            value: self.value.clone(),
        }
    }
}

impl Clone for LiteralValue {
    fn clone(&self) -> Self {
        match self {
            LiteralValue::IntValue(v) => LiteralValue::IntValue(*v),
            LiteralValue::BooleanValue(v) => LiteralValue::BooleanValue(*v),
            LiteralValue::StringValue(v) => LiteralValue::StringValue(v.clone()),
        }
    }
}

impl Clone for SymbolNode {
    fn clone(&self) -> Self {
        SymbolNode { symbol_name: self.symbol_name.clone() }
    }
}

impl Clone for ConsCell {
    fn clone(&self) -> Self {
        ConsCell {
            type_: self.type_,
            car: self.car.clone(),
            cdr: self.cdr.clone(),
        }
    }
}

impl Clone for Function {
    fn clone(&self) -> Self {
        Function {
            param_symbol_names: self.param_symbol_names.clone(),
            body: self.body.clone(),
        }
    }
}

impl Clone for Object {
    fn clone(&self) -> Self {
        Object {
            marked: self.marked,
            type_: self.type_,
            value: self.value.clone(),
        }
    }
}

impl Clone for ObjectValue {
    fn clone(&self) -> Self {
        match self {
            ObjectValue::IntValue(v) => ObjectValue::IntValue(*v),
            ObjectValue::StringValue(v) => ObjectValue::StringValue(v.clone()),
            ObjectValue::BoolValue(v) => ObjectValue::BoolValue(*v),
            ObjectValue::ListValue(v) => ObjectValue::ListValue(v.clone()),
            ObjectValue::FunctionValue(v) => ObjectValue::FunctionValue(v.clone()),
        }
    }
}

impl Clone for Binding {
    fn clone(&self) -> Self {
        Binding {
            symbol_name: self.symbol_name.clone(),
            value: self.value.clone(),
        }
    }
}

impl Clone for Env {
    fn clone(&self) -> Self {
        Env {
            bindings: std::array::from_fn(|i| self.bindings[i].clone()),
            parent: self.parent.clone(),
        }
    }
}

// =================================================
//   helpers
// =================================================

fn empty_binding() -> Binding {
    Binding {
        symbol_name: String::new(),
        value: None,
    }
}

fn empty_env() -> Env {
    Env {
        bindings: std::array::from_fn(|_| empty_binding()),
        parent: None,
    }
}

fn empty_object() -> Object {
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

fn is_op_char(c: u8) -> bool {
    matches!(
        c,
        b'+' | b'-' | b'*' | b'/' | b'%' | b'|' | b'&' | b'=' | b'<' | b'>'
    )
}

fn is_alpha(c: u8) -> bool {
    (c >= b'a' && c <= b'z') || (c >= b'A' && c <= b'Z')
}

fn is_digit(c: u8) -> bool {
    c >= b'0' && c <= b'9'
}

fn is_alnum(c: u8) -> bool {
    is_alpha(c) || is_digit(c)
}

fn is_space(c: u8) -> bool {
    c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c
}

// =================================================
//   tokenizer
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

pub fn next(source: &str, state: &mut ParseState) {
    let bytes = source.as_bytes();
    let len = bytes.len() as i32;

    // Skip whitespace
    while state.pos < len {
        let c = bytes[state.pos as usize];
        if is_space(c) {
            state.pos += 1;
        } else {
            break;
        }
    }

    let new_token: Token;

    if state.pos >= len {
        new_token = Token {
            kind: TokenKind::Eof,
            next: None,
            val: 0,
            str: "\0".to_string(),
        };
    } else {
        let c = bytes[state.pos as usize];
        if c == b'(' {
            new_token = Token {
                kind: TokenKind::LParen,
                next: None,
                val: 0,
                str: "(".to_string(),
            };
            state.pos += 1;
        } else if c == b')' {
            new_token = Token {
                kind: TokenKind::RParen,
                next: None,
                val: 0,
                str: ")".to_string(),
            };
            state.pos += 1;
        } else if c == b'\'' {
            new_token = Token {
                kind: TokenKind::Quote,
                next: None,
                val: 0,
                str: "'".to_string(),
            };
            state.pos += 1;
        } else if is_alpha(c) || is_op_char(c) {
            let start = state.pos as usize;
            while state.pos < len {
                let cc = bytes[state.pos as usize];
                if is_alnum(cc) || is_op_char(cc) {
                    state.pos += 1;
                } else {
                    break;
                }
            }
            let s = std::str::from_utf8(&bytes[start..state.pos as usize])
                .unwrap_or("")
                .to_string();
            if s == "true" {
                new_token = Token {
                    kind: TokenKind::True,
                    next: None,
                    val: 0,
                    str: String::new(),
                };
            } else if s == "false" {
                new_token = Token {
                    kind: TokenKind::False,
                    next: None,
                    val: 0,
                    str: String::new(),
                };
            } else {
                new_token = Token {
                    kind: TokenKind::Symbol,
                    next: None,
                    val: 0,
                    str: s,
                };
            }
        } else if is_digit(c) {
            let start = state.pos as usize;
            while state.pos < len && is_digit(bytes[state.pos as usize]) {
                state.pos += 1;
            }
            let s = std::str::from_utf8(&bytes[start..state.pos as usize]).unwrap_or("0");
            let val: i32 = s.parse().unwrap_or(0);
            new_token = Token {
                kind: TokenKind::Digit,
                next: None,
                val,
                str: String::new(),
            };
        } else if c == b'"' {
            state.pos += 1;
            let start = state.pos as usize;
            while state.pos < len && bytes[state.pos as usize] != b'"' {
                state.pos += 1;
            }
            let s = std::str::from_utf8(&bytes[start..state.pos as usize])
                .unwrap_or("")
                .to_string();
            if state.pos < len && bytes[state.pos as usize] == b'"' {
                state.pos += 1;
            }
            new_token = Token {
                kind: TokenKind::String,
                next: None,
                val: 0,
                str: s,
            };
        } else if c == b';' {
            // comment to end of line
            while state.pos < len && bytes[state.pos as usize] != b'\n' {
                state.pos += 1;
            }
            next(source, state);
            return;
        } else {
            // unexpected character; mimic C exit but use panic for unrecoverable error
            eprintln!("Unexpected token: {}", c as char);
            std::process::exit(1);
        }
    }

    // Replace the current token (we don't need to maintain the linked list,
    // since `state.token` is what is consulted for `match`).
    state.token = Some(Box::new(new_token));
}

// =================================================
//   parser
// =================================================

fn append_expression_to_list(list: &mut Option<Box<ExpressionList>>, expr: ExpressionNode) {
    let new_node = Box::new(ExpressionList {
        expression: Some(Box::new(expr)),
        next: None,
    });
    if list.is_none() {
        *list = Some(new_node);
        return;
    }
    let mut cur: &mut ExpressionList = list.as_deref_mut().unwrap();
    while cur.next.is_some() {
        cur = cur.next.as_deref_mut().unwrap();
    }
    cur.next = Some(new_node);
}

fn parse_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    if match_token(state, TokenKind::LParen) == 1 {
        return parse_symbolic_expression(source, state);
    } else if match_token(state, TokenKind::Quote) == 1 {
        return parse_list_expression(source, state);
    } else if match_token(state, TokenKind::Symbol) == 1 {
        return parse_symbol_expression(source, state);
    } else if match_token(state, TokenKind::Digit) == 1
        || match_token(state, TokenKind::String) == 1
        || match_token(state, TokenKind::True) == 1
        || match_token(state, TokenKind::False) == 1
    {
        return parse_literal_expression(source, state);
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

fn parse_symbolic_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    let mut sym_exp = SymbolicExpNode { expressions: None };
    next(source, state); // eat '('
    while match_token(state, TokenKind::RParen) != 1 {
        let item = parse_expression(source, state);
        append_expression_to_list(&mut sym_exp.expressions, item);
    }
    next(source, state); // eat ')'
    ExpressionNode {
        type_: ExpressionType::SymbolicExp,
        data: ExpressionData::SymbolicExp(Some(Box::new(sym_exp))),
    }
}

fn parse_list_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    let mut list_node = ListNode { expressions: None };
    next(source, state); // eat quote
    next(source, state); // eat '('
    while match_token(state, TokenKind::RParen) != 1 {
        let item = parse_expression(source, state);
        append_expression_to_list(&mut list_node.expressions, item);
    }
    next(source, state); // eat ')'
    ExpressionNode {
        type_: ExpressionType::List,
        data: ExpressionData::List(Some(Box::new(list_node))),
    }
}

fn parse_symbol_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    let name = state
        .token
        .as_ref()
        .map(|t| t.str.clone())
        .unwrap_or_default();
    next(source, state);
    ExpressionNode {
        type_: ExpressionType::Symbol,
        data: ExpressionData::Symbol(Some(Box::new(SymbolNode { symbol_name: name }))),
    }
}

fn parse_literal_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    let lit_node = if match_token(state, TokenKind::Digit) == 1 {
        let v = state.token.as_ref().map(|t| t.val).unwrap_or(0);
        let node = LiteralNode {
            type_: LiteralType::Integer,
            value: LiteralValue::IntValue(v),
        };
        next(source, state);
        node
    } else if match_token(state, TokenKind::String) == 1 {
        let s = state
            .token
            .as_ref()
            .map(|t| t.str.clone())
            .unwrap_or_default();
        let node = LiteralNode {
            type_: LiteralType::String,
            value: LiteralValue::StringValue(s),
        };
        next(source, state);
        node
    } else if match_token(state, TokenKind::True) == 1 {
        let node = LiteralNode {
            type_: LiteralType::Boolean,
            value: LiteralValue::BooleanValue(true),
        };
        next(source, state);
        node
    } else if match_token(state, TokenKind::False) == 1 {
        let node = LiteralNode {
            type_: LiteralType::Boolean,
            value: LiteralValue::BooleanValue(false),
        };
        next(source, state);
        node
    } else {
        let s = state
            .token
            .as_ref()
            .map(|t| t.str.clone())
            .unwrap_or_default();
        eprintln!("Unexpected token: {}", s);
        std::process::exit(1);
    };
    ExpressionNode {
        type_: ExpressionType::Literal,
        data: ExpressionData::Literal(Some(Box::new(lit_node))),
    }
}

pub fn parse(source: &str, state: &mut ParseState, result: &mut ParseResult) {
    next(source, state);
    let mut program = ProgramNode { expressions: None };
    while match_token(state, TokenKind::Eof) != 1 {
        let expr = parse_expression(source, state);
        append_expression_to_list(&mut program.expressions, expr);
    }
    result.program = Some(Box::new(program));
}

// =================================================
//   object helpers
// =================================================

fn bool_val(obj: &Object) -> bool {
    match obj.type_ {
        ObjectType::Bool => match &obj.value {
            ObjectValue::BoolValue(v) => *v != 0,
            _ => false,
        },
        ObjectType::Nil => false,
        _ => true,
    }
}

fn obj_eq(op1: &Object, op2: &Object) -> bool {
    let same_kind = matches!(
        (op1.type_, op2.type_),
        (ObjectType::Integer, ObjectType::Integer)
            | (ObjectType::String, ObjectType::String)
            | (ObjectType::Bool, ObjectType::Bool)
            | (ObjectType::List, ObjectType::List)
            | (ObjectType::Nil, ObjectType::Nil)
            | (ObjectType::Function, ObjectType::Function)
    );
    if !same_kind {
        return false;
    }
    match op1.type_ {
        ObjectType::Integer => match (&op1.value, &op2.value) {
            (ObjectValue::IntValue(a), ObjectValue::IntValue(b)) => a == b,
            _ => false,
        },
        ObjectType::String => match (&op1.value, &op2.value) {
            (ObjectValue::StringValue(a), ObjectValue::StringValue(b)) => a == b,
            _ => false,
        },
        ObjectType::Bool => match (&op1.value, &op2.value) {
            (ObjectValue::BoolValue(a), ObjectValue::BoolValue(b)) => a == b,
            _ => false,
        },
        ObjectType::List => false, // C compares pointers; freshly built lists are never equal
        ObjectType::Nil => true,
        ObjectType::Function => false,
    }
}

fn is_last_cons_cell(cell: &ConsCell) -> bool {
    match &cell.cdr {
        Some(o) => matches!(o.type_, ObjectType::Nil),
        None => true,
    }
}

pub fn stringify_object(obj: &Object) -> String {
    match obj.type_ {
        ObjectType::Integer => match &obj.value {
            ObjectValue::IntValue(v) => v.to_string(),
            _ => "0".to_string(),
        },
        ObjectType::String => match &obj.value {
            ObjectValue::StringValue(v) => v.clone(),
            _ => String::new(),
        },
        ObjectType::Bool => match &obj.value {
            ObjectValue::BoolValue(v) => {
                if *v != 0 {
                    "T".to_string()
                } else {
                    "F".to_string()
                }
            }
            _ => "F".to_string(),
        },
        ObjectType::List => {
            let mut s = String::from("(");
            if let ObjectValue::ListValue(Some(first)) = &obj.value {
                let mut current: &ConsCell = first.as_ref();
                loop {
                    if let Some(car) = &current.car {
                        s.push_str(&stringify_object(car));
                    }
                    if is_last_cons_cell(current) {
                        break;
                    }
                    s.push(' ');
                    // move to next cell via cdr
                    if let Some(cdr) = &current.cdr {
                        if let ObjectValue::ListValue(Some(next_cell)) = &cdr.value {
                            current = next_cell.as_ref();
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
            s.push(')');
            s
        }
        ObjectType::Nil => "nil".to_string(),
        ObjectType::Function => "<function>".to_string(),
    }
}

// =================================================
//   defined functions
// =================================================

fn defined_add(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            let a = if let ObjectValue::IntValue(v) = &op1.value { *v } else { 0 };
            let b = if let ObjectValue::IntValue(v) = &op2.value { *v } else { 0 };
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(a.wrapping_add(b));
        }
        (ObjectType::String, ObjectType::String) => {
            let a = if let ObjectValue::StringValue(v) = &op1.value { v.clone() } else { String::new() };
            let b = if let ObjectValue::StringValue(v) = &op2.value { v.clone() } else { String::new() };
            evaluated.type_ = ObjectType::String;
            evaluated.value = ObjectValue::StringValue(a + &b);
        }
        _ => {
            eprintln!("Type error: operands for + must be integers or strings.");
            std::process::exit(1);
        }
    }
}

fn defined_sub(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            let a = if let ObjectValue::IntValue(v) = &op1.value { *v } else { 0 };
            let b = if let ObjectValue::IntValue(v) = &op2.value { *v } else { 0 };
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(a.wrapping_sub(b));
        }
        _ => {
            eprintln!("Type error: operands for - must be integers.");
            std::process::exit(1);
        }
    }
}

fn defined_mul(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            let a = if let ObjectValue::IntValue(v) = &op1.value { *v } else { 0 };
            let b = if let ObjectValue::IntValue(v) = &op2.value { *v } else { 0 };
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(a.wrapping_mul(b));
        }
        _ => {
            eprintln!("Type error: operands for * must be integers.");
            std::process::exit(1);
        }
    }
}

fn defined_div(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            let a = if let ObjectValue::IntValue(v) = &op1.value { *v } else { 0 };
            let b = if let ObjectValue::IntValue(v) = &op2.value { *v } else { 0 };
            if b == 0 {
                eprintln!("Division by zero.");
                std::process::exit(1);
            }
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(a / b);
        }
        _ => {
            eprintln!("Type error: operands for / must be integers.");
            std::process::exit(1);
        }
    }
}

fn defined_mod(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            let a = if let ObjectValue::IntValue(v) = &op1.value { *v } else { 0 };
            let b = if let ObjectValue::IntValue(v) = &op2.value { *v } else { 0 };
            if b == 0 {
                eprintln!("Modulo by zero.");
                std::process::exit(1);
            }
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(a % b);
        }
        _ => {
            eprintln!("Type error: operands for % must be integers.");
            std::process::exit(1);
        }
    }
}

fn defined_lt(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            let a = if let ObjectValue::IntValue(v) = &op1.value { *v } else { 0 };
            let b = if let ObjectValue::IntValue(v) = &op2.value { *v } else { 0 };
            evaluated.type_ = ObjectType::Bool;
            evaluated.value = ObjectValue::BoolValue(if a < b { 1 } else { 0 });
        }
        _ => {
            eprintln!("Type error: operands for < must be integers.");
            std::process::exit(1);
        }
    }
}

fn defined_gt(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            let a = if let ObjectValue::IntValue(v) = &op1.value { *v } else { 0 };
            let b = if let ObjectValue::IntValue(v) = &op2.value { *v } else { 0 };
            evaluated.type_ = ObjectType::Bool;
            evaluated.value = ObjectValue::BoolValue(if a > b { 1 } else { 0 });
        }
        _ => {
            eprintln!("Type error: operands for > must be integers.");
            std::process::exit(1);
        }
    }
}

fn defined_eq(op1: &Object, op2: &Object, evaluated: &mut Object) {
    evaluated.type_ = ObjectType::Bool;
    evaluated.value = ObjectValue::BoolValue(if obj_eq(op1, op2) { 1 } else { 0 });
}

fn defined_not(op: &Object, evaluated: &mut Object) {
    if !matches!(op.type_, ObjectType::Bool) {
        eprintln!("Type error: not operand must be boolean.");
        std::process::exit(1);
    }
    let v = if let ObjectValue::BoolValue(v) = &op.value { *v } else { 0 };
    evaluated.type_ = ObjectType::Bool;
    evaluated.value = ObjectValue::BoolValue(if v != 0 { 0 } else { 1 });
}

fn defined_car(op: &Object, evaluated: &mut Object) {
    if !matches!(op.type_, ObjectType::List) {
        eprintln!("Type error: car operand must be list.");
        std::process::exit(1);
    }
    if let ObjectValue::ListValue(Some(cell)) = &op.value {
        if let Some(car) = &cell.car {
            *evaluated = (**car).clone();
        }
    }
}

fn defined_cdr(op: &Object, evaluated: &mut Object) {
    if !matches!(op.type_, ObjectType::List) {
        eprintln!("Type error: cdr operand must be list.");
        std::process::exit(1);
    }
    if let ObjectValue::ListValue(Some(cell)) = &op.value {
        if let Some(cdr) = &cell.cdr {
            *evaluated = (**cdr).clone();
        }
    }
}

fn defined_cons(op1: &Object, op2: &Object, evaluated: &mut Object) {
    let new_cell;
    match op2.type_ {
        ObjectType::List => {
            new_cell = ConsCell {
                type_: ConsCellType::Cell,
                car: Some(Box::new(op1.clone())),
                cdr: Some(Box::new(op2.clone())),
            };
        }
        ObjectType::Nil => {
            new_cell = ConsCell {
                type_: ConsCellType::Cell,
                car: Some(Box::new(op1.clone())),
                cdr: Some(Box::new(op2.clone())),
            };
        }
        _ => {
            // wrap op2 into its own list
            let inner_cell = ConsCell {
                type_: ConsCellType::Cell,
                car: Some(Box::new(op2.clone())),
                cdr: Some(Box::new(nil_object())),
            };
            let cdr_obj = Object {
                marked: false,
                type_: ObjectType::List,
                value: ObjectValue::ListValue(Some(Box::new(inner_cell))),
            };
            new_cell = ConsCell {
                type_: ConsCellType::Cell,
                car: Some(Box::new(op1.clone())),
                cdr: Some(Box::new(cdr_obj)),
            };
        }
    }
    evaluated.type_ = ObjectType::List;
    evaluated.value = ObjectValue::ListValue(Some(Box::new(new_cell)));
}

fn defined_split(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if !matches!(op1.type_, ObjectType::String) {
        eprintln!("Type error: split first operand must be string.");
        std::process::exit(1);
    }
    if !matches!(op2.type_, ObjectType::String) {
        eprintln!("Type error: split second operand must be string.");
        std::process::exit(1);
    }
    let s = if let ObjectValue::StringValue(v) = &op1.value { v.clone() } else { String::new() };
    let sep = if let ObjectValue::StringValue(v) = &op2.value { v.clone() } else { String::new() };

    let pieces: Vec<String> = if sep.is_empty() {
        // list of characters
        s.chars().map(|c| c.to_string()).collect()
    } else {
        // strtok-like behaviour: skip empty pieces, treat each char in sep as a delimiter
        let sep_chars: std::collections::HashSet<char> = sep.chars().collect();
        let mut out = Vec::new();
        let mut cur = String::new();
        for ch in s.chars() {
            if sep_chars.contains(&ch) {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            } else {
                cur.push(ch);
            }
        }
        if !cur.is_empty() {
            out.push(cur);
        }
        out
    };

    if pieces.is_empty() {
        evaluated.type_ = ObjectType::Nil;
        return;
    }

    // build cons-cell list
    let mut head: Option<Box<ConsCell>> = None;
    for piece in pieces.into_iter().rev() {
        let car_obj = Object {
            marked: false,
            type_: ObjectType::String,
            value: ObjectValue::StringValue(piece),
        };
        let cdr_obj = match head {
            None => Object {
                marked: false,
                type_: ObjectType::Nil,
                value: ObjectValue::IntValue(0),
            },
            Some(cell) => Object {
                marked: false,
                type_: ObjectType::List,
                value: ObjectValue::ListValue(Some(cell)),
            },
        };
        let cell_type = if matches!(cdr_obj.type_, ObjectType::Nil) {
            ConsCellType::Cell
        } else {
            ConsCellType::Cell
        };
        head = Some(Box::new(ConsCell {
            type_: cell_type,
            car: Some(Box::new(car_obj)),
            cdr: Some(Box::new(cdr_obj)),
        }));
    }

    evaluated.type_ = ObjectType::List;
    evaluated.value = ObjectValue::ListValue(head);
}

fn defined_list_ref(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if !matches!(op1.type_, ObjectType::List) {
        eprintln!("Type error: list-ref first operand must be list.");
        std::process::exit(1);
    }
    if !matches!(op2.type_, ObjectType::Integer) {
        eprintln!("Type error: list-ref second operand must be integer.");
        std::process::exit(1);
    }
    let index = if let ObjectValue::IntValue(v) = &op2.value { *v } else { 0 };
    if let ObjectValue::ListValue(Some(cell0)) = &op1.value {
        let mut current: &ConsCell = cell0.as_ref();
        for _ in 0..index {
            if let Some(cdr) = &current.cdr {
                if matches!(cdr.type_, ObjectType::Nil) {
                    eprintln!("Index out of range.");
                    std::process::exit(1);
                }
                if let ObjectValue::ListValue(Some(next_cell)) = &cdr.value {
                    current = next_cell.as_ref();
                } else {
                    eprintln!("Index out of range.");
                    std::process::exit(1);
                }
            } else {
                eprintln!("Index out of range.");
                std::process::exit(1);
            }
        }
        if let Some(car) = &current.car {
            *evaluated = (**car).clone();
        }
    }
}

fn defined_remove_whitespaces(op1: &Object, evaluated: &mut Object) {
    if !matches!(op1.type_, ObjectType::String) {
        eprintln!("Type error: remove-whitespaces operand must be string.");
        std::process::exit(1);
    }
    let s = if let ObjectValue::StringValue(v) = &op1.value { v.clone() } else { String::new() };
    let result: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    evaluated.type_ = ObjectType::String;
    evaluated.value = ObjectValue::StringValue(result);
}

#[allow(dead_code)]
fn defined_pop(op: &Object, evaluated: &mut Object) {
    if matches!(op.type_, ObjectType::Nil) {
        evaluated.type_ = ObjectType::Nil;
        evaluated.value = ObjectValue::IntValue(0);
        return;
    }
    if !matches!(op.type_, ObjectType::List) {
        eprintln!("Type error: pop operand must be list.");
        std::process::exit(1);
    }
    if let ObjectValue::ListValue(Some(cell0)) = &op.value {
        let mut current: &ConsCell = cell0.as_ref();
        loop {
            if is_last_cons_cell(current) {
                if let Some(car) = &current.car {
                    *evaluated = (**car).clone();
                }
                break;
            }
            if let Some(cdr) = &current.cdr {
                if let ObjectValue::ListValue(Some(next_cell)) = &cdr.value {
                    current = next_cell.as_ref();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }
}

fn defined_length(op: &Object, evaluated: &mut Object) {
    match op.type_ {
        ObjectType::Nil => {
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(0);
        }
        ObjectType::List => {
            let mut length = 1i32;
            if let ObjectValue::ListValue(Some(cell0)) = &op.value {
                let mut current: &ConsCell = cell0.as_ref();
                loop {
                    if is_last_cons_cell(current) {
                        break;
                    }
                    length += 1;
                    if let Some(cdr) = &current.cdr {
                        if let ObjectValue::ListValue(Some(next_cell)) = &cdr.value {
                            current = next_cell.as_ref();
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            } else {
                length = 0;
            }
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(length);
        }
        ObjectType::String => {
            let len = if let ObjectValue::StringValue(v) = &op.value {
                v.len() as i32
            } else {
                0
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

fn defined_is_int_string(op: &Object, evaluated: &mut Object) {
    let result = if matches!(op.type_, ObjectType::String) {
        if let ObjectValue::StringValue(s) = &op.value {
            !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
        } else {
            false
        }
    } else {
        false
    };
    evaluated.type_ = ObjectType::Bool;
    evaluated.value = ObjectValue::BoolValue(if result { 1 } else { 0 });
}

fn defined_parse_int(op: &Object, evaluated: &mut Object) {
    if !matches!(op.type_, ObjectType::String) {
        eprintln!("Type error: parse-int operand must be string.");
        std::process::exit(1);
    }
    let s = if let ObjectValue::StringValue(v) = &op.value { v.clone() } else { String::new() };
    if !s.chars().all(|c| c.is_ascii_digit()) {
        eprintln!("Type error: parse-int operand must be string of digits.");
        std::process::exit(1);
    }
    let v: i32 = s.parse().unwrap_or(0);
    evaluated.type_ = ObjectType::Integer;
    evaluated.value = ObjectValue::IntValue(v);
}

fn defined_string_ref(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if !matches!(op1.type_, ObjectType::String) {
        eprintln!("Type error: string-ref first operand must be string.");
        std::process::exit(1);
    }
    if !matches!(op2.type_, ObjectType::Integer) {
        eprintln!("Type error: string-ref second operand must be integer.");
        std::process::exit(1);
    }
    let s = if let ObjectValue::StringValue(v) = &op1.value { v.clone() } else { String::new() };
    let idx = if let ObjectValue::IntValue(v) = &op2.value { *v } else { 0 };
    let bytes = s.as_bytes();
    if idx < 0 || (idx as usize) >= bytes.len() {
        eprintln!("Index out of range.");
        std::process::exit(1);
    }
    let ch = bytes[idx as usize];
    evaluated.type_ = ObjectType::String;
    evaluated.value = ObjectValue::StringValue((ch as char).to_string());
}

// =================================================
//   environment helpers
// =================================================

fn set_object_to_env(env: &mut Env, symbol_name: &str, obj: Object) {
    let mut empty_idx: Option<usize> = None;
    for i in 0..MAX_BINDINGS {
        if env.bindings[i].symbol_name.is_empty() {
            empty_idx = Some(i);
            break;
        }
        if env.bindings[i].symbol_name == symbol_name {
            env.bindings[i].value = Some(Box::new(obj));
            return;
        }
    }
    if let Some(i) = empty_idx {
        env.bindings[i].symbol_name = symbol_name.to_string();
        env.bindings[i].value = Some(Box::new(obj));
    }
}

fn lookup_in_env(env: &Env, symbol_name: &str) -> Option<Object> {
    for i in 0..MAX_BINDINGS {
        if env.bindings[i].symbol_name.is_empty() {
            break;
        }
        if env.bindings[i].symbol_name == symbol_name {
            if let Some(v) = &env.bindings[i].value {
                return Some((**v).clone());
            }
        }
    }
    if let Some(parent) = &env.parent {
        return lookup_in_env(parent, symbol_name);
    }
    None
}

// =================================================
//   evaluator
// =================================================

fn evaluate_literal_expression(expression: &ExpressionNode, evaluated: &mut Object) {
    if let ExpressionData::Literal(Some(lit)) = &expression.data {
        match &lit.value {
            LiteralValue::IntValue(v) => {
                evaluated.type_ = ObjectType::Integer;
                evaluated.value = ObjectValue::IntValue(*v);
            }
            LiteralValue::StringValue(v) => {
                evaluated.type_ = ObjectType::String;
                evaluated.value = ObjectValue::StringValue(v.clone());
            }
            LiteralValue::BooleanValue(v) => {
                evaluated.type_ = ObjectType::Bool;
                evaluated.value = ObjectValue::BoolValue(if *v { 1 } else { 0 });
            }
        }
    }
}

fn evaluate_symbol_expression(
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
    if let Some(obj) = lookup_in_env(env, &name) {
        *evaluated = obj;
    } else {
        eprintln!("Undefined symbol: {}", name);
        std::process::exit(1);
    }
}

fn evaluate_list_expression(
    expression: &ExpressionNode,
    evaluated: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
    let expressions_opt = match &expression.data {
        ExpressionData::List(Some(list)) => list.expressions.as_deref(),
        _ => None,
    };
    if expressions_opt.is_none() {
        evaluated.type_ = ObjectType::Nil;
        evaluated.value = ObjectValue::IntValue(0);
        return;
    }

    // collect all evaluated items first
    let mut items: Vec<Object> = Vec::new();
    let mut cur = expressions_opt;
    while let Some(node) = cur {
        if let Some(expr) = &node.expression {
            let mut item = empty_object();
            evaluate_expression(expr, &mut item, env, context);
            items.push(item);
        }
        cur = node.next.as_deref();
    }

    if items.is_empty() {
        evaluated.type_ = ObjectType::Nil;
        evaluated.value = ObjectValue::IntValue(0);
        return;
    }

    // Build cons cells
    let mut head: Option<Box<ConsCell>> = None;
    for item in items.into_iter().rev() {
        let cdr_obj = match head {
            None => Object {
                marked: false,
                type_: ObjectType::Nil,
                value: ObjectValue::IntValue(0),
            },
            Some(cell) => Object {
                marked: false,
                type_: ObjectType::List,
                value: ObjectValue::ListValue(Some(cell)),
            },
        };
        let cell_type = if matches!(cdr_obj.type_, ObjectType::Nil) {
            ConsCellType::Nil
        } else {
            ConsCellType::Cell
        };
        head = Some(Box::new(ConsCell {
            type_: cell_type,
            car: Some(Box::new(item)),
            cdr: Some(Box::new(cdr_obj)),
        }));
    }

    evaluated.type_ = ObjectType::List;
    evaluated.value = ObjectValue::ListValue(head);
}

fn get_nth_expr<'a>(
    list: Option<&'a ExpressionList>,
    n: usize,
) -> Option<&'a ExpressionNode> {
    let mut cur = list;
    let mut i = 0usize;
    while let Some(node) = cur {
        if i == n {
            return node.expression.as_deref();
        }
        i += 1;
        cur = node.next.as_deref();
    }
    None
}

fn evaluate_symbolic_expression(
    expression: &ExpressionNode,
    evaluated: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
    // Get the inner expression list, prefer SymbolicExp variant; some C code reads .list,
    // but at this point the type tag is EXP_SYMBOLIC_EXP; the C union shares storage.
    let expressions_opt: Option<&ExpressionList> = match &expression.data {
        ExpressionData::SymbolicExp(Some(s)) => s.expressions.as_deref(),
        ExpressionData::List(Some(l)) => l.expressions.as_deref(),
        _ => None,
    };

    let expressions = match expressions_opt {
        Some(e) => e,
        None => {
            evaluated.type_ = ObjectType::Nil;
            evaluated.value = ObjectValue::IntValue(0);
            return;
        }
    };

    let head_expr = match &expressions.expression {
        Some(e) => e.as_ref(),
        None => {
            evaluated.type_ = ObjectType::Nil;
            evaluated.value = ObjectValue::IntValue(0);
            return;
        }
    };

    let symbol_name = match (&head_expr.type_, &head_expr.data) {
        (ExpressionType::Symbol, ExpressionData::Symbol(Some(sym))) => sym.symbol_name.clone(),
        _ => {
            eprintln!("S-exp must be started with symbol.");
            std::process::exit(1);
        }
    };

    match symbol_name.as_str() {
        "if" => {
            let cond = match get_nth_expr(Some(expressions), 1) {
                Some(c) => c.clone(),
                None => {
                    eprintln!("if must have condition.");
                    std::process::exit(1);
                }
            };
            let then_expr = match get_nth_expr(Some(expressions), 2) {
                Some(t) => t.clone(),
                None => {
                    eprintln!("if must have then clause.");
                    std::process::exit(1);
                }
            };
            let mut cond_obj = empty_object();
            evaluate_expression(&cond, &mut cond_obj, env, context);
            if bool_val(&cond_obj) {
                evaluate_expression(&then_expr, evaluated, env, context);
            } else if let Some(else_expr) = get_nth_expr(Some(expressions), 3) {
                let else_clone = else_expr.clone();
                evaluate_expression(&else_clone, evaluated, env, context);
            } else {
                evaluated.type_ = ObjectType::Nil;
                evaluated.value = ObjectValue::IntValue(0);
            }
        }
        "while" => {
            let cond = match get_nth_expr(Some(expressions), 1) {
                Some(c) => c.clone(),
                None => {
                    eprintln!("while must have condition.");
                    std::process::exit(1);
                }
            };
            let body = match get_nth_expr(Some(expressions), 2) {
                Some(t) => t.clone(),
                None => {
                    eprintln!("while must have body.");
                    std::process::exit(1);
                }
            };
            loop {
                let mut cond_obj = empty_object();
                evaluate_expression(&cond, &mut cond_obj, env, context);
                if bool_val(&cond_obj) {
                    evaluate_expression(&body, evaluated, env, context);
                } else {
                    evaluated.type_ = ObjectType::Nil;
                    evaluated.value = ObjectValue::IntValue(0);
                    break;
                }
            }
        }
        "=" => {
            let symbol_expr = match get_nth_expr(Some(expressions), 1) {
                Some(e) => e,
                None => {
                    eprintln!("assignment must have variable name.");
                    std::process::exit(1);
                }
            };
            let var_name = match (&symbol_expr.type_, &symbol_expr.data) {
                (ExpressionType::Symbol, ExpressionData::Symbol(Some(s))) => s.symbol_name.clone(),
                _ => {
                    eprintln!("Variable name must be symbol.");
                    std::process::exit(1);
                }
            };
            let value_expr = match get_nth_expr(Some(expressions), 2) {
                Some(e) => e.clone(),
                None => {
                    eprintln!("assignment must have expression.");
                    std::process::exit(1);
                }
            };
            let mut val_obj = empty_object();
            evaluate_expression(&value_expr, &mut val_obj, env, context);
            *evaluated = val_obj.clone();
            set_object_to_env(env, &var_name, val_obj);
        }
        "defun" => {
            let name_expr = match get_nth_expr(Some(expressions), 1) {
                Some(e) => e,
                None => {
                    eprintln!("Function must have a name.");
                    std::process::exit(1);
                }
            };
            let fn_name = match (&name_expr.type_, &name_expr.data) {
                (ExpressionType::Symbol, ExpressionData::Symbol(Some(s))) => s.symbol_name.clone(),
                _ => {
                    eprintln!("Function name must be symbol.");
                    std::process::exit(1);
                }
            };
            let params_expr = match get_nth_expr(Some(expressions), 2) {
                Some(e) => e,
                None => {
                    eprintln!("Function must have parameter.");
                    std::process::exit(1);
                }
            };
            let params_list = match (&params_expr.type_, &params_expr.data) {
                (ExpressionType::SymbolicExp, ExpressionData::SymbolicExp(Some(se))) => {
                    se.expressions.as_deref()
                }
                _ => {
                    eprintln!("Function parameter must be list.");
                    std::process::exit(1);
                }
            };
            let mut param_names: Vec<String> = Vec::new();
            let mut cur = params_list;
            while let Some(node) = cur {
                if let Some(e) = &node.expression {
                    match (&e.type_, &e.data) {
                        (ExpressionType::Symbol, ExpressionData::Symbol(Some(sym))) => {
                            param_names.push(sym.symbol_name.clone());
                        }
                        _ => {
                            eprintln!("Function parameter must be symbol.");
                            std::process::exit(1);
                        }
                    }
                }
                cur = node.next.as_deref();
            }

            let body_expr = match get_nth_expr(Some(expressions), 3) {
                Some(b) => b.clone(),
                None => {
                    eprintln!("Function must have body.");
                    std::process::exit(1);
                }
            };

            let func = Function {
                param_symbol_names: param_names,
                body: Some(Box::new(body_expr)),
            };
            evaluated.type_ = ObjectType::Function;
            evaluated.value = ObjectValue::FunctionValue(Some(Box::new(func)));

            set_object_to_env(env, &fn_name, evaluated.clone());
        }
        "+" => {
            let a = eval_nth(expressions, 1, env, context);
            let b = eval_nth(expressions, 2, env, context);
            defined_add(&a, &b, evaluated);
        }
        "-" => {
            let a = eval_nth(expressions, 1, env, context);
            let b = eval_nth(expressions, 2, env, context);
            defined_sub(&a, &b, evaluated);
        }
        "*" => {
            let a = eval_nth(expressions, 1, env, context);
            let b = eval_nth(expressions, 2, env, context);
            defined_mul(&a, &b, evaluated);
        }
        "/" => {
            let a = eval_nth(expressions, 1, env, context);
            let b = eval_nth(expressions, 2, env, context);
            defined_div(&a, &b, evaluated);
        }
        "%" => {
            let a = eval_nth(expressions, 1, env, context);
            let b = eval_nth(expressions, 2, env, context);
            defined_mod(&a, &b, evaluated);
        }
        "||" => {
            // short circuit OR
            let mut cur = expressions.next.as_deref();
            while let Some(node) = cur {
                if let Some(e) = &node.expression {
                    let expr_clone = (**e).clone();
                    let mut operand = empty_object();
                    evaluate_expression(&expr_clone, &mut operand, env, context);
                    if bool_val(&operand) {
                        evaluated.type_ = ObjectType::Bool;
                        evaluated.value = ObjectValue::BoolValue(1);
                        return;
                    }
                }
                cur = node.next.as_deref();
            }
            evaluated.type_ = ObjectType::Bool;
            evaluated.value = ObjectValue::BoolValue(0);
        }
        "&&" => {
            let mut cur = expressions.next.as_deref();
            while let Some(node) = cur {
                if let Some(e) = &node.expression {
                    let expr_clone = (**e).clone();
                    let mut operand = empty_object();
                    evaluate_expression(&expr_clone, &mut operand, env, context);
                    if !bool_val(&operand) {
                        evaluated.type_ = ObjectType::Bool;
                        evaluated.value = ObjectValue::BoolValue(0);
                        return;
                    }
                }
                cur = node.next.as_deref();
            }
            evaluated.type_ = ObjectType::Bool;
            evaluated.value = ObjectValue::BoolValue(1);
        }
        "<" => {
            let a = eval_nth(expressions, 1, env, context);
            let b = eval_nth(expressions, 2, env, context);
            defined_lt(&a, &b, evaluated);
        }
        ">" => {
            let a = eval_nth(expressions, 1, env, context);
            let b = eval_nth(expressions, 2, env, context);
            defined_gt(&a, &b, evaluated);
        }
        "eq" => {
            let a = eval_nth(expressions, 1, env, context);
            let b = eval_nth(expressions, 2, env, context);
            defined_eq(&a, &b, evaluated);
        }
        "not" => {
            let a = eval_nth(expressions, 1, env, context);
            defined_not(&a, evaluated);
        }
        "print" => {
            let a = eval_nth(expressions, 1, env, context);
            let s = stringify_object(&a);
            println!("{}", s);
            evaluated.type_ = ObjectType::Nil;
            evaluated.value = ObjectValue::IntValue(0);
        }
        "car" => {
            let a = eval_nth(expressions, 1, env, context);
            defined_car(&a, evaluated);
        }
        "cdr" => {
            let a = eval_nth(expressions, 1, env, context);
            defined_cdr(&a, evaluated);
        }
        "cons" => {
            let a = eval_nth(expressions, 1, env, context);
            let b = eval_nth(expressions, 2, env, context);
            defined_cons(&a, &b, evaluated);
        }
        "readline" => {
            use std::io::BufRead;
            let stdin = std::io::stdin();
            let mut buf = String::new();
            match stdin.lock().read_line(&mut buf) {
                Ok(0) => {
                    evaluated.type_ = ObjectType::Nil;
                    evaluated.value = ObjectValue::IntValue(0);
                }
                Ok(_) => {
                    if buf.ends_with('\n') {
                        buf.pop();
                        if buf.ends_with('\r') {
                            buf.pop();
                        }
                    }
                    evaluated.type_ = ObjectType::String;
                    evaluated.value = ObjectValue::StringValue(buf);
                }
                Err(_) => {
                    evaluated.type_ = ObjectType::Nil;
                    evaluated.value = ObjectValue::IntValue(0);
                }
            }
        }
        "split" => {
            let a = eval_nth(expressions, 1, env, context);
            let b = eval_nth(expressions, 2, env, context);
            defined_split(&a, &b, evaluated);
        }
        "list-ref" => {
            let a = eval_nth(expressions, 1, env, context);
            let b = eval_nth(expressions, 2, env, context);
            defined_list_ref(&a, &b, evaluated);
        }
        "progn" => {
            let mut cur = expressions.next.as_deref();
            let mut last = empty_object();
            let mut had_any = false;
            while let Some(node) = cur {
                if let Some(e) = &node.expression {
                    let expr_clone = (**e).clone();
                    let mut operand = empty_object();
                    evaluate_expression(&expr_clone, &mut operand, env, context);
                    last = operand;
                    had_any = true;
                }
                cur = node.next.as_deref();
            }
            if had_any {
                *evaluated = last;
            } else {
                evaluated.type_ = ObjectType::Nil;
                evaluated.value = ObjectValue::IntValue(0);
            }
        }
        "remove-whitespaces" => {
            let a = eval_nth(expressions, 1, env, context);
            defined_remove_whitespaces(&a, evaluated);
        }
        "pop" => {
            let a = eval_nth(expressions, 1, env, context);
            let lst_name: Option<String> = match get_nth_expr(Some(expressions), 1) {
                Some(e) => match (&e.type_, &e.data) {
                    (ExpressionType::Symbol, ExpressionData::Symbol(Some(s))) => {
                        Some(s.symbol_name.clone())
                    }
                    _ => None,
                },
                None => None,
            };
            pop_op(a, evaluated, env, lst_name);
        }
        "push" => {
            // (push lst val): the C version mutates the binding's list in place.
            // We replicate that here by building a new list and rebinding it.
            let val = eval_nth(expressions, 2, env, context);
            let lst = eval_nth(expressions, 1, env, context);
            // Determine if first argument is a symbol that we can rebind.
            let lst_name: Option<String> = match get_nth_expr(Some(expressions), 1) {
                Some(e) => match (&e.type_, &e.data) {
                    (ExpressionType::Symbol, ExpressionData::Symbol(Some(s))) => {
                        Some(s.symbol_name.clone())
                    }
                    _ => None,
                },
                None => None,
            };
            push_op(&lst, &val, evaluated, env, lst_name);
        }
        "length" => {
            let a = eval_nth(expressions, 1, env, context);
            defined_length(&a, evaluated);
        }
        "is-int-string" => {
            let a = eval_nth(expressions, 1, env, context);
            defined_is_int_string(&a, evaluated);
        }
        "parse-int" => {
            let a = eval_nth(expressions, 1, env, context);
            defined_parse_int(&a, evaluated);
        }
        "string-ref" => {
            let a = eval_nth(expressions, 1, env, context);
            let b = eval_nth(expressions, 2, env, context);
            defined_string_ref(&a, &b, evaluated);
        }
        _ => {
            // user-defined function call
            if let Some(func_obj) = lookup_in_env(env, &symbol_name) {
                if let ObjectValue::FunctionValue(Some(func)) = &func_obj.value {
                    // Build new env
                    let mut new_env = empty_env();
                    new_env.parent = Some(Box::new(env.clone()));

                    // Bind parameters
                    let mut cur = expressions.next.as_deref();
                    for pname in &func.param_symbol_names {
                        let arg_expr_opt = cur.and_then(|n| n.expression.as_deref());
                        if let Some(arg_expr) = arg_expr_opt {
                            let arg_clone = arg_expr.clone();
                            let mut param_val = empty_object();
                            evaluate_expression(&arg_clone, &mut param_val, env, context);
                            set_object_to_env(&mut new_env, pname, param_val);
                        }
                        cur = cur.and_then(|n| n.next.as_deref());
                    }

                    if let Some(body) = &func.body {
                        let body_clone = (**body).clone();
                        evaluate_expression(&body_clone, evaluated, &mut new_env, context);
                    }
                    return;
                }
            }
            eprintln!("Undefined function: {}", symbol_name);
            std::process::exit(1);
        }
    }
}

fn eval_nth(
    expressions: &ExpressionList,
    n: usize,
    env: &mut Env,
    context: &mut AllocatorContext,
) -> Object {
    let mut result = empty_object();
    if let Some(expr) = get_nth_expr(Some(expressions), n) {
        let expr_clone = expr.clone();
        evaluate_expression(&expr_clone, &mut result, env, context);
    }
    result
}

fn pop_op(mut lst: Object, evaluated: &mut Object, env: &mut Env, lst_name: Option<String>) {
    if matches!(lst.type_, ObjectType::Nil) {
        evaluated.type_ = ObjectType::Nil;
        evaluated.value = ObjectValue::IntValue(0);
        return;
    }
    if !matches!(lst.type_, ObjectType::List) {
        eprintln!("Type error: pop operand must be list.");
        std::process::exit(1);
    }

    // Walk to the last cons cell and capture the last value, then truncate.
    let popped: Option<Object>;
    let became_empty: bool;
    if let ObjectValue::ListValue(Some(head_cell)) = &mut lst.value {
        // Single element list?
        if matches!(
            head_cell.cdr.as_deref().map(|o| o.type_),
            Some(ObjectType::Nil) | None
        ) {
            popped = head_cell.car.as_deref().cloned();
            became_empty = true;
        } else {
            // Walk to the second-to-last cell
            became_empty = false;
            popped = pop_walk(head_cell.as_mut());
        }
    } else {
        evaluated.type_ = ObjectType::Nil;
        evaluated.value = ObjectValue::IntValue(0);
        return;
    }

    if let Some(p) = popped {
        *evaluated = p;
    } else {
        evaluated.type_ = ObjectType::Nil;
        evaluated.value = ObjectValue::IntValue(0);
    }

    if let Some(name) = lst_name {
        if became_empty {
            // Set binding to nil
            set_object_to_env(
                env,
                &name,
                Object {
                    marked: false,
                    type_: ObjectType::Nil,
                    value: ObjectValue::IntValue(0),
                },
            );
        } else {
            set_object_to_env(env, &name, lst);
        }
    }
}

fn pop_walk(head: &mut ConsCell) -> Option<Object> {
    // Walk until cdr.list_value's cdr is nil. Then truncate.
    let mut current: &mut ConsCell = head;
    loop {
        // Check if next cell is the last
        let next_is_last: bool = match &current.cdr {
            Some(o) => match &o.value {
                ObjectValue::ListValue(Some(next_cell)) => match &next_cell.cdr {
                    Some(c) => matches!(c.type_, ObjectType::Nil),
                    None => true,
                },
                _ => false,
            },
            None => false,
        };
        if next_is_last {
            // popped value is from current.cdr.list_value.car
            if let Some(cdr) = current.cdr.as_mut() {
                if let ObjectValue::ListValue(Some(next_cell)) = &mut cdr.value {
                    let popped = next_cell.car.as_deref().cloned();
                    // Truncate: replace current.cdr with nil
                    current.cdr = Some(Box::new(Object {
                        marked: false,
                        type_: ObjectType::Nil,
                        value: ObjectValue::IntValue(0),
                    }));
                    current.type_ = ConsCellType::Nil;
                    return popped;
                }
            }
            return None;
        }
        // Move forward
        let cdr_box = match current.cdr.as_mut() {
            Some(c) => c,
            None => return None,
        };
        if let ObjectValue::ListValue(Some(next_cell)) = &mut cdr_box.value {
            current = next_cell.as_mut();
        } else {
            return None;
        }
    }
}

fn push_op(
    lst: &Object,
    val: &Object,
    evaluated: &mut Object,
    env: &mut Env,
    lst_name: Option<String>,
) {
    if matches!(lst.type_, ObjectType::Nil) {
        // Replace binding's nil with a singleton list containing val.
        if let Some(name) = lst_name {
            let new_cell = ConsCell {
                type_: ConsCellType::Cell,
                car: Some(Box::new(val.clone())),
                cdr: Some(Box::new(Object {
                    marked: false,
                    type_: ObjectType::Nil,
                    value: ObjectValue::IntValue(0),
                })),
            };
            let new_list = Object {
                marked: false,
                type_: ObjectType::List,
                value: ObjectValue::ListValue(Some(Box::new(new_cell))),
            };
            set_object_to_env(env, &name, new_list);
        }
        *evaluated = val.clone();
        return;
    }

    if !matches!(lst.type_, ObjectType::List) {
        eprintln!("Type error: push second operand must be list.");
        std::process::exit(1);
    }

    // Build new list with val appended
    let mut new_list = lst.clone();
    append_to_list(&mut new_list, val.clone());
    if let Some(name) = lst_name {
        set_object_to_env(env, &name, new_list);
    }
    *evaluated = val.clone();
}

fn append_to_list(list_obj: &mut Object, val: Object) {
    if let ObjectValue::ListValue(Some(cell)) = &mut list_obj.value {
        let mut current: &mut ConsCell = cell.as_mut();
        loop {
            let is_last = match &current.cdr {
                Some(c) => matches!(c.type_, ObjectType::Nil),
                None => true,
            };
            if is_last {
                let new_cell = ConsCell {
                    type_: ConsCellType::Cell,
                    car: Some(Box::new(val)),
                    cdr: Some(Box::new(Object {
                        marked: false,
                        type_: ObjectType::Nil,
                        value: ObjectValue::IntValue(0),
                    })),
                };
                let cdr_obj = Object {
                    marked: false,
                    type_: ObjectType::List,
                    value: ObjectValue::ListValue(Some(Box::new(new_cell))),
                };
                current.cdr = Some(Box::new(cdr_obj));
                current.type_ = ConsCellType::Cell;
                return;
            }
            // move forward
            let cdr_box = current.cdr.as_mut().unwrap();
            if let ObjectValue::ListValue(Some(next_cell)) = &mut cdr_box.value {
                current = next_cell.as_mut();
            } else {
                return;
            }
        }
    }
}

pub fn evaluate_expression(
    expression: &ExpressionNode,
    result: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
    match expression.type_ {
        ExpressionType::List => evaluate_list_expression(expression, result, env, context),
        ExpressionType::SymbolicExp => evaluate_symbolic_expression(expression, result, env, context),
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
    if let Some(program) = &result.program {
        let mut env = empty_env();
        let mut context = init_allocator();
        let mut cur = program.expressions.as_deref();
        while let Some(node) = cur {
            if let Some(expr) = &node.expression {
                let expr_clone = (**expr).clone();
                let mut evaluated = empty_object();
                evaluate_expression(&expr_clone, &mut evaluated, &mut env, &mut context);
            }
            cur = node.next.as_deref();
        }
    }
}

pub fn init_env(env: &mut Env) {
    env.parent = None;
    for i in 0..MAX_BINDINGS {
        env.bindings[i].symbol_name = String::new();
        env.bindings[i].value = None;
    }
}

pub fn init_allocator() -> AllocatorContext {
    let stack_objects: [Option<Box<Object>>; OBJECT_NUMBER] = std::array::from_fn(|_| None);
    AllocatorContext {
        gc_less_mode: 0,
        stack: Some(Box::new(ObjectStack {
            objects: stack_objects,
            top: -1,
        })),
        memory_pool: None,
        free_bitmap: [0u8; FREE_BITMAP_SIZE],
    }
}

pub fn allocate(_context: &mut AllocatorContext, _env: &mut Env) -> Option<Box<Object>> {
    Some(Box::new(empty_object()))
}
