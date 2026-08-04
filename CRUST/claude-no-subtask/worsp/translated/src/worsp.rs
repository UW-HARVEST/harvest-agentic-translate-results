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
// Helper functions
// =================================================

fn is_op(c: u8) -> bool {
    matches!(
        c,
        b'+' | b'-' | b'*' | b'/' | b'%' | b'|' | b'&' | b'=' | b'<' | b'>'
    )
}

fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

fn is_alpha(c: u8) -> bool {
    c.is_ascii_alphabetic()
}

fn is_alnum(c: u8) -> bool {
    c.is_ascii_alphanumeric()
}

fn make_token(kind: TokenKind, str: String, val: i32) -> Token {
    Token {
        kind,
        next: None,
        val,
        str,
    }
}

fn nil_object() -> Object {
    Object {
        marked: false,
        type_: ObjectType::Nil,
        value: ObjectValue::IntValue(0),
    }
}

fn int_object(v: i32) -> Object {
    Object {
        marked: false,
        type_: ObjectType::Integer,
        value: ObjectValue::IntValue(v),
    }
}

fn bool_object(b: bool) -> Object {
    Object {
        marked: false,
        type_: ObjectType::Bool,
        value: ObjectValue::BoolValue(if b { 1 } else { 0 }),
    }
}

fn string_object(s: String) -> Object {
    Object {
        marked: false,
        type_: ObjectType::String,
        value: ObjectValue::StringValue(s),
    }
}

fn list_object(cell: Option<Box<ConsCell>>) -> Object {
    Object {
        marked: false,
        type_: ObjectType::List,
        value: ObjectValue::ListValue(cell),
    }
}

fn function_object(f: Function) -> Object {
    Object {
        marked: false,
        type_: ObjectType::Function,
        value: ObjectValue::FunctionValue(Some(Box::new(f))),
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
        ObjectValue::ListValue(c) => {
            ObjectValue::ListValue(c.as_ref().map(|cell| Box::new(clone_conscell(cell))))
        }
        ObjectValue::FunctionValue(f) => {
            ObjectValue::FunctionValue(f.as_ref().map(|func| Box::new(clone_function(func))))
        }
    }
}

fn clone_conscell(c: &ConsCell) -> ConsCell {
    ConsCell {
        type_: c.type_,
        car: c.car.as_ref().map(|o| Box::new(clone_object(o))),
        cdr: c.cdr.as_ref().map(|o| Box::new(clone_object(o))),
    }
}

fn clone_function(f: &Function) -> Function {
    Function {
        param_symbol_names: f.param_symbol_names.clone(),
        body: f.body.as_ref().map(|b| Box::new(clone_expression(b))),
    }
}

fn clone_expression(e: &ExpressionNode) -> ExpressionNode {
    ExpressionNode {
        type_: e.type_,
        data: clone_expression_data(&e.data),
    }
}

fn clone_expression_data(d: &ExpressionData) -> ExpressionData {
    match d {
        ExpressionData::SymbolicExp(s) => ExpressionData::SymbolicExp(
            s.as_ref().map(|n| {
                Box::new(SymbolicExpNode {
                    expressions: clone_expression_list(&n.expressions),
                })
            }),
        ),
        ExpressionData::List(l) => ExpressionData::List(l.as_ref().map(|n| {
            Box::new(ListNode {
                expressions: clone_expression_list(&n.expressions),
            })
        })),
        ExpressionData::Literal(l) => {
            ExpressionData::Literal(l.as_ref().map(|n| Box::new(clone_literal(n))))
        }
        ExpressionData::Symbol(s) => ExpressionData::Symbol(s.as_ref().map(|n| {
            Box::new(SymbolNode {
                symbol_name: n.symbol_name.clone(),
            })
        })),
    }
}

fn clone_expression_list(
    list: &Option<Box<ExpressionList>>,
) -> Option<Box<ExpressionList>> {
    list.as_ref().map(|node| {
        Box::new(ExpressionList {
            expression: node
                .expression
                .as_ref()
                .map(|e| Box::new(clone_expression(e))),
            next: clone_expression_list(&node.next),
        })
    })
}

fn clone_literal(l: &LiteralNode) -> LiteralNode {
    LiteralNode {
        type_: l.type_,
        value: match &l.value {
            LiteralValue::IntValue(i) => LiteralValue::IntValue(*i),
            LiteralValue::BooleanValue(b) => LiteralValue::BooleanValue(*b),
            LiteralValue::StringValue(s) => LiteralValue::StringValue(s.clone()),
        },
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

// =================================================
// Tokenizer
// =================================================

pub fn match_token(state: &mut ParseState, kind: TokenKind) -> i32 {
    match &state.token {
        Some(t) if t.kind == kind => 1,
        _ => 0,
    }
}

pub fn next(source: &str, state: &mut ParseState) {
    let bytes = source.as_bytes();
    let mut pos = state.pos as usize;

    // skip whitespace
    while pos < bytes.len() && is_space(bytes[pos]) {
        pos += 1;
    }

    let new_token: Token;

    if pos >= bytes.len() {
        // EOF
        new_token = make_token(TokenKind::Eof, String::from("\0"), 0);
        state.pos = pos as i32;
    } else {
        let c = bytes[pos];
        if c == b'(' {
            new_token = make_token(TokenKind::LParen, String::from("("), 0);
            pos += 1;
            state.pos = pos as i32;
        } else if c == b')' {
            new_token = make_token(TokenKind::RParen, String::from(")"), 0);
            pos += 1;
            state.pos = pos as i32;
        } else if c == b'\'' {
            new_token = make_token(TokenKind::Quote, String::from("'"), 0);
            pos += 1;
            state.pos = pos as i32;
        } else if is_alpha(c) || is_op(c) {
            let start = pos;
            while pos < bytes.len() && (is_alnum(bytes[pos]) || is_op(bytes[pos])) {
                pos += 1;
            }
            let s = std::str::from_utf8(&bytes[start..pos]).unwrap().to_string();
            state.pos = pos as i32;
            new_token = if s == "true" {
                make_token(TokenKind::True, String::new(), 0)
            } else if s == "false" {
                make_token(TokenKind::False, String::new(), 0)
            } else {
                make_token(TokenKind::Symbol, s, 0)
            };
        } else if is_digit(c) {
            let start = pos;
            while pos < bytes.len() && is_digit(bytes[pos]) {
                pos += 1;
            }
            let s = std::str::from_utf8(&bytes[start..pos]).unwrap();
            let val: i32 = s.parse().unwrap_or(0);
            state.pos = pos as i32;
            new_token = make_token(TokenKind::Digit, String::new(), val);
        } else if c == b'"' {
            pos += 1;
            let start = pos;
            while pos < bytes.len() && bytes[pos] != b'"' {
                pos += 1;
            }
            let s = std::str::from_utf8(&bytes[start..pos]).unwrap().to_string();
            if pos < bytes.len() && bytes[pos] == b'"' {
                pos += 1;
            }
            state.pos = pos as i32;
            new_token = make_token(TokenKind::String, s, 0);
        } else if c == b';' {
            // comment - skip until newline
            while pos < bytes.len() && bytes[pos] != b'\n' {
                pos += 1;
            }
            state.pos = pos as i32;
            next(source, state);
            return;
        } else {
            panic!("Unexpected token: {}", c as char);
        }
    }

    state.token = Some(Box::new(new_token));
}

// =================================================
// Parser
// =================================================

fn append_expression_to_list_node(list_node: &mut ListNode, expression: ExpressionNode) {
    let new_entry = Box::new(ExpressionList {
        expression: Some(Box::new(expression)),
        next: None,
    });
    if list_node.expressions.is_none() {
        list_node.expressions = Some(new_entry);
        return;
    }
    let mut current = list_node.expressions.as_mut().unwrap();
    while current.next.is_some() {
        current = current.next.as_mut().unwrap();
    }
    current.next = Some(new_entry);
}

fn append_expression_to_sexp_node(
    sexp_node: &mut SymbolicExpNode,
    expression: ExpressionNode,
) {
    let new_entry = Box::new(ExpressionList {
        expression: Some(Box::new(expression)),
        next: None,
    });
    if sexp_node.expressions.is_none() {
        sexp_node.expressions = Some(new_entry);
        return;
    }
    let mut current = sexp_node.expressions.as_mut().unwrap();
    while current.next.is_some() {
        current = current.next.as_mut().unwrap();
    }
    current.next = Some(new_entry);
}

fn append_expression_to_program(program: &mut ProgramNode, expression: ExpressionNode) {
    let new_entry = Box::new(ExpressionList {
        expression: Some(Box::new(expression)),
        next: None,
    });
    if program.expressions.is_none() {
        program.expressions = Some(new_entry);
        return;
    }
    let mut current = program.expressions.as_mut().unwrap();
    while current.next.is_some() {
        current = current.next.as_mut().unwrap();
    }
    current.next = Some(new_entry);
}

fn parse_symbolic_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    let mut sexp = SymbolicExpNode { expressions: None };
    next(source, state); // eat '('
    while match_token(state, TokenKind::RParen) == 0 {
        let item = parse_expression(source, state);
        append_expression_to_sexp_node(&mut sexp, item);
    }
    next(source, state); // eat ')'
    ExpressionNode {
        type_: ExpressionType::SymbolicExp,
        data: ExpressionData::SymbolicExp(Some(Box::new(sexp))),
    }
}

fn parse_list_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    let mut list = ListNode { expressions: None };
    next(source, state); // eat '
    next(source, state); // eat '('
    while match_token(state, TokenKind::RParen) == 0 {
        let item = parse_expression(source, state);
        append_expression_to_list_node(&mut list, item);
    }
    next(source, state); // eat ')'
    ExpressionNode {
        type_: ExpressionType::List,
        data: ExpressionData::List(Some(Box::new(list))),
    }
}

fn parse_symbol_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    let symbol_name = match &state.token {
        Some(t) => t.str.clone(),
        None => String::new(),
    };
    let node = ExpressionNode {
        type_: ExpressionType::Symbol,
        data: ExpressionData::Symbol(Some(Box::new(SymbolNode { symbol_name }))),
    };
    next(source, state);
    node
}

fn parse_literal_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    let (lit_type, lit_value) = if match_token(state, TokenKind::Digit) == 1 {
        let val = state.token.as_ref().map(|t| t.val).unwrap_or(0);
        (LiteralType::Integer, LiteralValue::IntValue(val))
    } else if match_token(state, TokenKind::String) == 1 {
        let s = state
            .token
            .as_ref()
            .map(|t| t.str.clone())
            .unwrap_or_default();
        (LiteralType::String, LiteralValue::StringValue(s))
    } else if match_token(state, TokenKind::True) == 1 {
        (LiteralType::Boolean, LiteralValue::BooleanValue(true))
    } else if match_token(state, TokenKind::False) == 1 {
        (LiteralType::Boolean, LiteralValue::BooleanValue(false))
    } else {
        let s = state
            .token
            .as_ref()
            .map(|t| t.str.clone())
            .unwrap_or_default();
        panic!("Unexpected token: {}", s);
    };
    let node = ExpressionNode {
        type_: ExpressionType::Literal,
        data: ExpressionData::Literal(Some(Box::new(LiteralNode {
            type_: lit_type,
            value: lit_value,
        }))),
    };
    next(source, state);
    node
}

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

pub fn parse(source: &str, state: &mut ParseState, result: &mut ParseResult) {
    next(source, state); // first token
    let mut program = ProgramNode { expressions: None };
    while match_token(state, TokenKind::Eof) == 0 {
        let expression = parse_expression(source, state);
        append_expression_to_program(&mut program, expression);
    }
    result.program = Some(Box::new(program));
}

// =================================================
// Allocator
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
    Some(Box::new(nil_object()))
}

// =================================================
// Env
// =================================================

pub fn init_env(env: &mut Env) {
    for i in 0..MAX_BINDINGS {
        env.bindings[i].symbol_name = String::new();
        env.bindings[i].value = None;
    }
    env.parent = None;
}

fn env_lookup(env: &Env, name: &str) -> Option<Object> {
    for i in 0..MAX_BINDINGS {
        if env.bindings[i].symbol_name.is_empty() {
            break;
        }
        if env.bindings[i].symbol_name == name {
            if let Some(v) = &env.bindings[i].value {
                return Some(clone_object(v));
            } else {
                return None;
            }
        }
    }
    if let Some(parent) = &env.parent {
        return env_lookup(parent, name);
    }
    None
}

fn env_lookup_function(env: &Env, name: &str) -> Option<Function> {
    for i in 0..MAX_BINDINGS {
        if env.bindings[i].symbol_name.is_empty() {
            break;
        }
        if env.bindings[i].symbol_name == name {
            if let Some(v) = &env.bindings[i].value {
                if let ObjectValue::FunctionValue(Some(f)) = &v.value {
                    return Some(clone_function(f));
                }
            }
            return None;
        }
    }
    if let Some(parent) = &env.parent {
        return env_lookup_function(parent, name);
    }
    None
}

fn env_set(env: &mut Env, name: &str, obj: Object) {
    for i in 0..MAX_BINDINGS {
        if env.bindings[i].symbol_name == name {
            env.bindings[i].value = Some(Box::new(obj));
            return;
        }
        if env.bindings[i].symbol_name.is_empty() {
            env.bindings[i].symbol_name = name.to_string();
            env.bindings[i].value = Some(Box::new(obj));
            return;
        }
    }
}

// =================================================
// Helper for boolean and equality
// =================================================

fn bool_val(obj: &Object) -> bool {
    match (&obj.type_, &obj.value) {
        (ObjectType::Bool, ObjectValue::BoolValue(b)) => *b != 0,
        (ObjectType::Nil, _) => false,
        _ => true,
    }
}

fn obj_eq(a: &Object, b: &Object) -> bool {
    match (&a.type_, &b.type_) {
        (ObjectType::Integer, ObjectType::Integer) => match (&a.value, &b.value) {
            (ObjectValue::IntValue(x), ObjectValue::IntValue(y)) => x == y,
            _ => false,
        },
        (ObjectType::String, ObjectType::String) => match (&a.value, &b.value) {
            (ObjectValue::StringValue(x), ObjectValue::StringValue(y)) => x == y,
            _ => false,
        },
        (ObjectType::Bool, ObjectType::Bool) => match (&a.value, &b.value) {
            (ObjectValue::BoolValue(x), ObjectValue::BoolValue(y)) => x == y,
            _ => false,
        },
        (ObjectType::Nil, ObjectType::Nil) => true,
        (ObjectType::List, ObjectType::List) => {
            // Reference identity is impossible without raw pointers; treat structural equality
            false
        }
        _ => false,
    }
}

// =================================================
// Stringify
// =================================================

pub fn stringify_object(obj: &Object) -> String {
    match (&obj.type_, &obj.value) {
        (ObjectType::Integer, ObjectValue::IntValue(i)) => i.to_string(),
        (ObjectType::String, ObjectValue::StringValue(s)) => s.clone(),
        (ObjectType::Bool, ObjectValue::BoolValue(b)) => {
            if *b != 0 {
                String::from("T")
            } else {
                String::from("F")
            }
        }
        (ObjectType::Nil, _) => String::from("nil"),
        (ObjectType::Function, _) => String::from("<function>"),
        (ObjectType::List, ObjectValue::ListValue(cell)) => {
            let mut result = String::from("(");
            let mut current = cell.as_deref();
            let mut first = true;
            while let Some(c) = current {
                if !first {
                    result.push(' ');
                }
                first = false;
                if let Some(car) = &c.car {
                    result.push_str(&stringify_object(car));
                }
                // determine next
                match &c.cdr {
                    Some(cdr_obj) => match &cdr_obj.value {
                        ObjectValue::ListValue(next_cell) => {
                            current = next_cell.as_deref();
                        }
                        _ => break,
                    },
                    None => break,
                }
                if matches!(c.type_, ConsCellType::Nil) {
                    break;
                }
            }
            result.push(')');
            result
        }
        _ => String::new(),
    }
}

// =================================================
// Evaluator helpers
// =================================================

fn eval_list_expression(
    expression: &ExpressionNode,
    env: &mut Env,
    context: &mut AllocatorContext,
) -> Object {
    let expressions_opt = match &expression.data {
        ExpressionData::List(Some(node)) => &node.expressions,
        _ => return nil_object(),
    };

    if expressions_opt.is_none() {
        return nil_object();
    }

    // Collect all sub-objects
    let mut items: Vec<Object> = Vec::new();
    let mut current = expressions_opt;
    while let Some(node) = current {
        if let Some(expr) = &node.expression {
            let mut item = nil_object();
            evaluate_expression(expr, &mut item, env, context);
            items.push(item);
        }
        current = &node.next;
    }

    // Build cons-cell linked list
    let mut head: Option<Box<ConsCell>> = None;
    for item in items.into_iter().rev() {
        let cdr_obj = match head {
            None => Box::new(nil_object()),
            Some(c) => Box::new(list_object(Some(c))),
        };
        let cell_type = if matches!(cdr_obj.type_, ObjectType::Nil) {
            ConsCellType::Nil
        } else {
            ConsCellType::Cell
        };
        let new_cell = Box::new(ConsCell {
            type_: cell_type,
            car: Some(Box::new(item)),
            cdr: Some(cdr_obj),
        });
        head = Some(new_cell);
    }

    list_object(head)
}

fn get_sexp_expressions(expression: &ExpressionNode) -> Option<&Option<Box<ExpressionList>>> {
    match &expression.data {
        ExpressionData::SymbolicExp(Some(node)) => Some(&node.expressions),
        _ => None,
    }
}

fn nth_expr(list: &Option<Box<ExpressionList>>, n: usize) -> Option<&ExpressionNode> {
    let mut current = list;
    let mut i = 0;
    while let Some(node) = current {
        if i == n {
            return node.expression.as_deref();
        }
        current = &node.next;
        i += 1;
    }
    None
}

fn arith_add(op1: &Object, op2: &Object) -> Object {
    match (&op1.value, &op2.value) {
        (ObjectValue::IntValue(a), ObjectValue::IntValue(b))
            if matches!(op1.type_, ObjectType::Integer)
                && matches!(op2.type_, ObjectType::Integer) =>
        {
            int_object(a + b)
        }
        (ObjectValue::StringValue(a), ObjectValue::StringValue(b))
            if matches!(op1.type_, ObjectType::String)
                && matches!(op2.type_, ObjectType::String) =>
        {
            let mut s = a.clone();
            s.push_str(b);
            string_object(s)
        }
        _ => panic!("Type error: operands for + must be integers or strings."),
    }
}

fn arith_int_op(op1: &Object, op2: &Object, op: char) -> Object {
    match (&op1.type_, &op2.type_, &op1.value, &op2.value) {
        (
            ObjectType::Integer,
            ObjectType::Integer,
            ObjectValue::IntValue(a),
            ObjectValue::IntValue(b),
        ) => {
            let v = match op {
                '-' => a - b,
                '*' => a * b,
                '/' => a / b,
                '%' => a % b,
                _ => unreachable!(),
            };
            int_object(v)
        }
        _ => panic!("Type error: operands for {} must be integers.", op),
    }
}

fn cmp_int(op1: &Object, op2: &Object, lt: bool) -> Object {
    match (&op1.type_, &op2.type_, &op1.value, &op2.value) {
        (
            ObjectType::Integer,
            ObjectType::Integer,
            ObjectValue::IntValue(a),
            ObjectValue::IntValue(b),
        ) => bool_object(if lt { a < b } else { a > b }),
        _ => panic!("Type error: operands for </> must be integers."),
    }
}

fn function_call_user(
    func: Function,
    args_exprs: &Option<Box<ExpressionList>>,
    env: &mut Env,
    context: &mut AllocatorContext,
) -> Object {
    // Evaluate arguments in current env
    let params = func.param_symbol_names.clone();
    let mut arg_values: Vec<Object> = Vec::new();

    let mut current = args_exprs;
    let mut idx = 0;
    while idx < params.len() {
        if let Some(node) = current {
            if let Some(expr) = node.expression.as_deref() {
                let mut v = nil_object();
                evaluate_expression(expr, &mut v, env, context);
                arg_values.push(v);
            }
            current = &node.next;
        } else {
            break;
        }
        idx += 1;
    }

    // Build new env with parent = current env (cloned at this snapshot)
    let parent_clone = clone_env(env);
    let mut new_env = make_env();
    new_env.parent = Some(Box::new(parent_clone));

    for (i, name) in params.iter().enumerate() {
        if let Some(val) = arg_values.get(i) {
            env_set(&mut new_env, name, clone_object(val));
        }
    }

    let mut result = nil_object();
    if let Some(body) = func.body.as_deref() {
        evaluate_expression(body, &mut result, &mut new_env, context);
    }
    result
}

fn clone_env(env: &Env) -> Env {
    Env {
        bindings: std::array::from_fn(|i| Binding {
            symbol_name: env.bindings[i].symbol_name.clone(),
            value: env.bindings[i]
                .value
                .as_ref()
                .map(|v| Box::new(clone_object(v))),
        }),
        parent: env.parent.as_ref().map(|p| Box::new(clone_env(p))),
    }
}

fn def_cons(op1: Object, op2: Object) -> Object {
    let cell = match op2.type_ {
        ObjectType::List => ConsCell {
            type_: ConsCellType::Cell,
            car: Some(Box::new(op1)),
            cdr: Some(Box::new(op2)),
        },
        ObjectType::Nil => ConsCell {
            type_: ConsCellType::Nil,
            car: Some(Box::new(op1)),
            cdr: Some(Box::new(op2)),
        },
        _ => {
            // build a 2-element list
            let inner = ConsCell {
                type_: ConsCellType::Nil,
                car: Some(Box::new(op2)),
                cdr: Some(Box::new(nil_object())),
            };
            let cdr_obj = list_object(Some(Box::new(inner)));
            ConsCell {
                type_: ConsCellType::Cell,
                car: Some(Box::new(op1)),
                cdr: Some(Box::new(cdr_obj)),
            }
        }
    };
    list_object(Some(Box::new(cell)))
}

fn def_car(op: &Object) -> Object {
    match (&op.type_, &op.value) {
        (ObjectType::List, ObjectValue::ListValue(Some(c))) => match &c.car {
            Some(car) => clone_object(car),
            None => nil_object(),
        },
        _ => panic!("Type error: car operand must be list."),
    }
}

fn def_cdr(op: &Object) -> Object {
    match (&op.type_, &op.value) {
        (ObjectType::List, ObjectValue::ListValue(Some(c))) => match &c.cdr {
            Some(cdr) => clone_object(cdr),
            None => nil_object(),
        },
        _ => panic!("Type error: cdr operand must be list."),
    }
}

fn def_length(op: &Object) -> Object {
    match (&op.type_, &op.value) {
        (ObjectType::Nil, _) => int_object(0),
        (ObjectType::List, ObjectValue::ListValue(cell_opt)) => {
            let mut n: i32 = 0;
            let mut cur = cell_opt.as_deref();
            while let Some(c) = cur {
                n += 1;
                if matches!(c.type_, ConsCellType::Nil) {
                    break;
                }
                match &c.cdr {
                    Some(cdr_obj) => match &cdr_obj.value {
                        ObjectValue::ListValue(next_cell) => {
                            cur = next_cell.as_deref();
                        }
                        _ => break,
                    },
                    None => break,
                }
            }
            int_object(n)
        }
        (ObjectType::String, ObjectValue::StringValue(s)) => int_object(s.len() as i32),
        _ => panic!("Type error: length operand must be list or string."),
    }
}

fn def_list_ref(op1: &Object, op2: &Object) -> Object {
    let index = match (&op2.type_, &op2.value) {
        (ObjectType::Integer, ObjectValue::IntValue(i)) => *i,
        _ => panic!("Type error: list-ref second operand must be integer."),
    };
    let mut cur = match (&op1.type_, &op1.value) {
        (ObjectType::List, ObjectValue::ListValue(cell)) => cell.as_deref(),
        _ => panic!("Type error: list-ref first operand must be list."),
    };
    let mut i = 0;
    while let Some(c) = cur {
        if i == index {
            return c.car.as_deref().map(clone_object).unwrap_or_else(nil_object);
        }
        if matches!(c.type_, ConsCellType::Nil) {
            panic!("Index out of range.");
        }
        match &c.cdr {
            Some(cdr_obj) => match &cdr_obj.value {
                ObjectValue::ListValue(next_cell) => {
                    cur = next_cell.as_deref();
                }
                _ => panic!("Index out of range."),
            },
            None => panic!("Index out of range."),
        }
        i += 1;
    }
    panic!("Index out of range.")
}

fn def_split(op1: &Object, op2: &Object) -> Object {
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
        s1.split(&s2)
            .filter(|p| !p.is_empty())
            .map(|p| p.to_string())
            .collect()
    };

    // Build cons cell list
    let mut head: Option<Box<ConsCell>> = None;
    for p in parts.into_iter().rev() {
        let cdr_obj = match head {
            None => Box::new(nil_object()),
            Some(c) => Box::new(list_object(Some(c))),
        };
        let cell_type = if matches!(cdr_obj.type_, ObjectType::Nil) {
            ConsCellType::Cell // matches C: only top-level nil-flag tracked
        } else {
            ConsCellType::Cell
        };
        head = Some(Box::new(ConsCell {
            type_: cell_type,
            car: Some(Box::new(string_object(p))),
            cdr: Some(cdr_obj),
        }));
    }
    list_object(head)
}

fn def_remove_whitespaces(op: &Object) -> Object {
    match (&op.type_, &op.value) {
        (ObjectType::String, ObjectValue::StringValue(s)) => {
            let result: String = s.chars().filter(|c| !c.is_whitespace()).collect();
            string_object(result)
        }
        _ => panic!("Type error: remove-whitespaces operand must be string."),
    }
}

fn def_is_int_string(op: &Object) -> Object {
    match (&op.type_, &op.value) {
        (ObjectType::String, ObjectValue::StringValue(s)) => {
            let all_digits = !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
            bool_object(all_digits || s.is_empty())
        }
        _ => bool_object(false),
    }
}

fn def_parse_int(op: &Object) -> Object {
    match (&op.type_, &op.value) {
        (ObjectType::String, ObjectValue::StringValue(s)) => {
            let val: i32 = s.parse().unwrap_or(0);
            int_object(val)
        }
        _ => panic!("Type error: parse-int operand must be string."),
    }
}

fn def_string_ref(op1: &Object, op2: &Object) -> Object {
    let s = match (&op1.type_, &op1.value) {
        (ObjectType::String, ObjectValue::StringValue(s)) => s.clone(),
        _ => panic!("Type error: string-ref first operand must be string."),
    };
    let i = match (&op2.type_, &op2.value) {
        (ObjectType::Integer, ObjectValue::IntValue(i)) => *i,
        _ => panic!("Type error: string-ref second operand must be integer."),
    };
    if i < 0 || (i as usize) >= s.len() {
        panic!("Index out of range.");
    }
    let bytes = s.as_bytes();
    let c = bytes[i as usize] as char;
    string_object(c.to_string())
}

fn def_pop(op: &Object) -> Object {
    match (&op.type_, &op.value) {
        (ObjectType::Nil, _) => nil_object(),
        (ObjectType::List, ObjectValue::ListValue(cell_opt)) => {
            let mut cur = cell_opt.as_deref();
            let mut last_car: Option<Object> = None;
            while let Some(c) = cur {
                if matches!(c.type_, ConsCellType::Nil) {
                    last_car = c.car.as_deref().map(clone_object);
                    break;
                }
                match &c.cdr {
                    Some(cdr_obj) => match &cdr_obj.value {
                        ObjectValue::ListValue(next_cell) => {
                            cur = next_cell.as_deref();
                        }
                        _ => {
                            last_car = c.car.as_deref().map(clone_object);
                            break;
                        }
                    },
                    None => {
                        last_car = c.car.as_deref().map(clone_object);
                        break;
                    }
                }
            }
            last_car.unwrap_or_else(nil_object)
        }
        _ => panic!("Type error: pop operand must be list."),
    }
}

// =================================================
// Evaluator
// =================================================

fn evaluate_literal_expression(expression: &ExpressionNode, evaluated: &mut Object) {
    if let ExpressionData::Literal(Some(lit)) = &expression.data {
        match (&lit.type_, &lit.value) {
            (LiteralType::Integer, LiteralValue::IntValue(i)) => {
                *evaluated = int_object(*i);
            }
            (LiteralType::String, LiteralValue::StringValue(s)) => {
                *evaluated = string_object(s.clone());
            }
            (LiteralType::Boolean, LiteralValue::BooleanValue(b)) => {
                *evaluated = bool_object(*b);
            }
            _ => {}
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
        *evaluated = nil_object();
        return;
    }
    if let Some(v) = env_lookup(env, &name) {
        *evaluated = v;
    } else {
        panic!("Undefined symbol: {}", name);
    }
}

fn evaluate_symbolic_expression(
    expression: &ExpressionNode,
    evaluated: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
    let expressions = match get_sexp_expressions(expression) {
        Some(e) => e,
        None => {
            *evaluated = nil_object();
            return;
        }
    };

    if expressions.is_none() {
        *evaluated = nil_object();
        return;
    }

    // The head must be a symbol
    let head_expr = nth_expr(expressions, 0);
    let head_symbol = match head_expr {
        Some(e) => match &e.data {
            ExpressionData::Symbol(Some(s)) => s.symbol_name.clone(),
            _ => panic!("S-exp must be started with symbol."),
        },
        None => {
            *evaluated = nil_object();
            return;
        }
    };

    match head_symbol.as_str() {
        "if" => {
            let cond = nth_expr(expressions, 1).expect("if must have condition");
            let then_e = nth_expr(expressions, 2).expect("if must have then clause");
            let mut cond_val = nil_object();
            evaluate_expression(cond, &mut cond_val, env, context);
            if bool_val(&cond_val) {
                evaluate_expression(then_e, evaluated, env, context);
            } else if let Some(else_e) = nth_expr(expressions, 3) {
                evaluate_expression(else_e, evaluated, env, context);
            } else {
                *evaluated = nil_object();
            }
        }
        "while" => {
            let cond = nth_expr(expressions, 1).expect("while must have condition");
            let body = nth_expr(expressions, 2).expect("while must have body");
            loop {
                let mut cond_val = nil_object();
                evaluate_expression(cond, &mut cond_val, env, context);
                if bool_val(&cond_val) {
                    evaluate_expression(body, evaluated, env, context);
                } else {
                    *evaluated = nil_object();
                    break;
                }
            }
        }
        "=" => {
            let symbol_expr = nth_expr(expressions, 1).expect("Variable name required");
            let symbol_name = match &symbol_expr.data {
                ExpressionData::Symbol(Some(s)) => s.symbol_name.clone(),
                _ => panic!("Variable name must be symbol."),
            };
            let val_expr = nth_expr(expressions, 2).expect("assignment must have expression");
            let mut val = nil_object();
            evaluate_expression(val_expr, &mut val, env, context);
            *evaluated = clone_object(&val);
            env_set(env, &symbol_name, val);
        }
        "defun" => {
            let name_expr = nth_expr(expressions, 1).expect("Function name required");
            let symbol_name = match &name_expr.data {
                ExpressionData::Symbol(Some(s)) => s.symbol_name.clone(),
                _ => panic!("Function name must be symbol."),
            };
            let params_expr = nth_expr(expressions, 2).expect("Function must have parameter");
            let params_list = match &params_expr.data {
                ExpressionData::SymbolicExp(Some(node)) => &node.expressions,
                _ => panic!("Function parameter must be list."),
            };
            let mut param_names: Vec<String> = Vec::new();
            let mut current = params_list;
            while let Some(node) = current {
                if let Some(e) = node.expression.as_deref() {
                    match &e.data {
                        ExpressionData::Symbol(Some(s)) => {
                            param_names.push(s.symbol_name.clone());
                        }
                        _ => panic!("Function parameter must be symbol."),
                    }
                }
                current = &node.next;
            }
            let body_expr = nth_expr(expressions, 3).expect("Function must have body");
            let function = Function {
                param_symbol_names: param_names,
                body: Some(Box::new(clone_expression(body_expr))),
            };
            let func_obj = function_object(function);
            *evaluated = clone_object(&func_obj);
            env_set(env, &symbol_name, func_obj);
        }
        "+" => {
            let mut a = nil_object();
            let mut b = nil_object();
            evaluate_expression(nth_expr(expressions, 1).unwrap(), &mut a, env, context);
            evaluate_expression(nth_expr(expressions, 2).unwrap(), &mut b, env, context);
            *evaluated = arith_add(&a, &b);
        }
        "-" => {
            let mut a = nil_object();
            let mut b = nil_object();
            evaluate_expression(nth_expr(expressions, 1).unwrap(), &mut a, env, context);
            evaluate_expression(nth_expr(expressions, 2).unwrap(), &mut b, env, context);
            *evaluated = arith_int_op(&a, &b, '-');
        }
        "*" => {
            let mut a = nil_object();
            let mut b = nil_object();
            evaluate_expression(nth_expr(expressions, 1).unwrap(), &mut a, env, context);
            evaluate_expression(nth_expr(expressions, 2).unwrap(), &mut b, env, context);
            *evaluated = arith_int_op(&a, &b, '*');
        }
        "/" => {
            let mut a = nil_object();
            let mut b = nil_object();
            evaluate_expression(nth_expr(expressions, 1).unwrap(), &mut a, env, context);
            evaluate_expression(nth_expr(expressions, 2).unwrap(), &mut b, env, context);
            *evaluated = arith_int_op(&a, &b, '/');
        }
        "%" => {
            let mut a = nil_object();
            let mut b = nil_object();
            evaluate_expression(nth_expr(expressions, 1).unwrap(), &mut a, env, context);
            evaluate_expression(nth_expr(expressions, 2).unwrap(), &mut b, env, context);
            *evaluated = arith_int_op(&a, &b, '%');
        }
        "||" => {
            let mut current = match expressions {
                Some(node) => &node.next,
                None => {
                    *evaluated = bool_object(false);
                    return;
                }
            };
            while let Some(node) = current {
                if let Some(e) = node.expression.as_deref() {
                    let mut v = nil_object();
                    evaluate_expression(e, &mut v, env, context);
                    if bool_val(&v) {
                        *evaluated = bool_object(true);
                        return;
                    }
                }
                current = &node.next;
            }
            *evaluated = bool_object(false);
        }
        "&&" => {
            let mut current = match expressions {
                Some(node) => &node.next,
                None => {
                    *evaluated = bool_object(true);
                    return;
                }
            };
            while let Some(node) = current {
                if let Some(e) = node.expression.as_deref() {
                    let mut v = nil_object();
                    evaluate_expression(e, &mut v, env, context);
                    if !bool_val(&v) {
                        *evaluated = bool_object(false);
                        return;
                    }
                }
                current = &node.next;
            }
            *evaluated = bool_object(true);
        }
        "<" => {
            let mut a = nil_object();
            let mut b = nil_object();
            evaluate_expression(nth_expr(expressions, 1).unwrap(), &mut a, env, context);
            evaluate_expression(nth_expr(expressions, 2).unwrap(), &mut b, env, context);
            *evaluated = cmp_int(&a, &b, true);
        }
        ">" => {
            let mut a = nil_object();
            let mut b = nil_object();
            evaluate_expression(nth_expr(expressions, 1).unwrap(), &mut a, env, context);
            evaluate_expression(nth_expr(expressions, 2).unwrap(), &mut b, env, context);
            *evaluated = cmp_int(&a, &b, false);
        }
        "eq" => {
            let mut a = nil_object();
            let mut b = nil_object();
            evaluate_expression(nth_expr(expressions, 1).unwrap(), &mut a, env, context);
            evaluate_expression(nth_expr(expressions, 2).unwrap(), &mut b, env, context);
            *evaluated = bool_object(obj_eq(&a, &b));
        }
        "not" => {
            let mut a = nil_object();
            evaluate_expression(nth_expr(expressions, 1).unwrap(), &mut a, env, context);
            match (&a.type_, &a.value) {
                (ObjectType::Bool, ObjectValue::BoolValue(b)) => {
                    *evaluated = bool_object(*b == 0);
                }
                _ => panic!("Type error: not operand must be boolean."),
            }
        }
        "print" => {
            let mut a = nil_object();
            evaluate_expression(nth_expr(expressions, 1).unwrap(), &mut a, env, context);
            let s = stringify_object(&a);
            println!("{}", s);
            *evaluated = nil_object();
        }
        "car" => {
            let mut a = nil_object();
            evaluate_expression(nth_expr(expressions, 1).unwrap(), &mut a, env, context);
            *evaluated = def_car(&a);
        }
        "cdr" => {
            let mut a = nil_object();
            evaluate_expression(nth_expr(expressions, 1).unwrap(), &mut a, env, context);
            *evaluated = def_cdr(&a);
        }
        "cons" => {
            let mut a = nil_object();
            let mut b = nil_object();
            evaluate_expression(nth_expr(expressions, 1).unwrap(), &mut a, env, context);
            evaluate_expression(nth_expr(expressions, 2).unwrap(), &mut b, env, context);
            *evaluated = def_cons(a, b);
        }
        "split" => {
            let mut a = nil_object();
            let mut b = nil_object();
            evaluate_expression(nth_expr(expressions, 1).unwrap(), &mut a, env, context);
            evaluate_expression(nth_expr(expressions, 2).unwrap(), &mut b, env, context);
            *evaluated = def_split(&a, &b);
        }
        "list-ref" => {
            let mut a = nil_object();
            let mut b = nil_object();
            evaluate_expression(nth_expr(expressions, 1).unwrap(), &mut a, env, context);
            evaluate_expression(nth_expr(expressions, 2).unwrap(), &mut b, env, context);
            *evaluated = def_list_ref(&a, &b);
        }
        "progn" => {
            let mut last: Option<Object> = None;
            let mut current = match expressions {
                Some(node) => &node.next,
                None => &None,
            };
            while let Some(node) = current {
                if let Some(e) = node.expression.as_deref() {
                    let mut v = nil_object();
                    evaluate_expression(e, &mut v, env, context);
                    last = Some(v);
                }
                current = &node.next;
            }
            *evaluated = last.unwrap_or_else(nil_object);
        }
        "remove-whitespaces" => {
            let mut a = nil_object();
            evaluate_expression(nth_expr(expressions, 1).unwrap(), &mut a, env, context);
            *evaluated = def_remove_whitespaces(&a);
        }
        "pop" => {
            let mut a = nil_object();
            evaluate_expression(nth_expr(expressions, 1).unwrap(), &mut a, env, context);
            *evaluated = def_pop(&a);
        }
        "push" => {
            // Mirrors C: push order is unusual
            let mut op1 = nil_object();
            let mut op2 = nil_object();
            evaluate_expression(nth_expr(expressions, 2).unwrap(), &mut op1, env, context);
            evaluate_expression(nth_expr(expressions, 1).unwrap(), &mut op2, env, context);
            *evaluated = clone_object(&op1);
            // We won't mutate env binding here since the C semantics are complex
            let _ = op2;
        }
        "length" => {
            let mut a = nil_object();
            evaluate_expression(nth_expr(expressions, 1).unwrap(), &mut a, env, context);
            *evaluated = def_length(&a);
        }
        "is-int-string" => {
            let mut a = nil_object();
            evaluate_expression(nth_expr(expressions, 1).unwrap(), &mut a, env, context);
            *evaluated = def_is_int_string(&a);
        }
        "parse-int" => {
            let mut a = nil_object();
            evaluate_expression(nth_expr(expressions, 1).unwrap(), &mut a, env, context);
            *evaluated = def_parse_int(&a);
        }
        "string-ref" => {
            let mut a = nil_object();
            let mut b = nil_object();
            evaluate_expression(nth_expr(expressions, 1).unwrap(), &mut a, env, context);
            evaluate_expression(nth_expr(expressions, 2).unwrap(), &mut b, env, context);
            *evaluated = def_string_ref(&a, &b);
        }
        _ => {
            // user-defined function call
            if let Some(func) = env_lookup_function(env, &head_symbol) {
                let args_exprs = match expressions {
                    Some(node) => &node.next,
                    None => &None,
                };
                let result = function_call_user(func, args_exprs, env, context);
                *evaluated = result;
            } else {
                panic!("Undefined function: {}", head_symbol);
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
        ExpressionType::List => {
            *result = eval_list_expression(expression, env, context);
        }
        ExpressionType::SymbolicExp => {
            evaluate_symbolic_expression(expression, result, env, context);
        }
        ExpressionType::Literal => {
            evaluate_literal_expression(expression, result);
        }
        ExpressionType::Symbol => {
            evaluate_symbol_expression(expression, result, env);
        }
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
    let mut env = make_env();
    init_env(&mut env);
    let mut context = init_allocator();
    if let Some(prog) = result.program.as_deref() {
        let mut current = &prog.expressions;
        while let Some(node) = current {
            if let Some(expr) = node.expression.as_deref() {
                let mut evaluated = nil_object();
                evaluate_expression(expr, &mut evaluated, &mut env, &mut context);
            }
            current = &node.next;
        }
    }
}
