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
//  helper functions
// =================================================

fn is_op_char(ch: u8) -> bool {
    matches!(
        ch,
        b'+' | b'-' | b'*' | b'/' | b'%' | b'|' | b'&' | b'=' | b'<' | b'>'
    )
}

fn is_alpha_char(ch: u8) -> bool {
    (ch >= b'a' && ch <= b'z') || (ch >= b'A' && ch <= b'Z')
}

fn is_alnum_char(ch: u8) -> bool {
    is_alpha_char(ch) || (ch >= b'0' && ch <= b'9')
}

fn is_digit_char(ch: u8) -> bool {
    ch >= b'0' && ch <= b'9'
}

fn is_space_char(ch: u8) -> bool {
    matches!(ch, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C)
}

fn current_kind(state: &ParseState) -> TokenKind {
    state
        .token
        .as_ref()
        .map(|t| t.kind)
        .unwrap_or(TokenKind::Eof)
}

fn make_nil_object() -> Object {
    Object {
        marked: false,
        type_: ObjectType::Nil,
        value: ObjectValue::IntValue(0),
    }
}

fn clone_object(o: &Object) -> Object {
    Object {
        marked: o.marked,
        type_: o.type_,
        value: clone_object_value(&o.value),
    }
}

fn clone_object_value(v: &ObjectValue) -> ObjectValue {
    match v {
        ObjectValue::IntValue(i) => ObjectValue::IntValue(*i),
        ObjectValue::StringValue(s) => ObjectValue::StringValue(s.clone()),
        ObjectValue::BoolValue(b) => ObjectValue::BoolValue(*b),
        ObjectValue::ListValue(opt) => ObjectValue::ListValue(
            opt.as_ref()
                .map(|c| Box::new(clone_conscell(c.as_ref()))),
        ),
        ObjectValue::FunctionValue(opt) => ObjectValue::FunctionValue(
            opt.as_ref()
                .map(|f| Box::new(clone_function(f.as_ref()))),
        ),
    }
}

fn clone_conscell(c: &ConsCell) -> ConsCell {
    ConsCell {
        type_: c.type_,
        car: c
            .car
            .as_ref()
            .map(|o| Box::new(clone_object(o.as_ref()))),
        cdr: c
            .cdr
            .as_ref()
            .map(|o| Box::new(clone_object(o.as_ref()))),
    }
}

fn clone_function(f: &Function) -> Function {
    Function {
        param_symbol_names: f.param_symbol_names.clone(),
        body: f
            .body
            .as_ref()
            .map(|b| Box::new(clone_expression_node(b.as_ref()))),
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
        ExpressionData::SymbolicExp(opt) => ExpressionData::SymbolicExp(
            opt.as_ref().map(|s| {
                Box::new(SymbolicExpNode {
                    expressions: clone_expression_list(s.expressions.as_deref()),
                })
            }),
        ),
        ExpressionData::List(opt) => ExpressionData::List(opt.as_ref().map(|l| {
            Box::new(ListNode {
                expressions: clone_expression_list(l.expressions.as_deref()),
            })
        })),
        ExpressionData::Literal(opt) => ExpressionData::Literal(
            opt.as_ref()
                .map(|l| Box::new(clone_literal_node(l.as_ref()))),
        ),
        ExpressionData::Symbol(opt) => ExpressionData::Symbol(opt.as_ref().map(|s| {
            Box::new(SymbolNode {
                symbol_name: s.symbol_name.clone(),
            })
        })),
    }
}

fn clone_expression_list(l: Option<&ExpressionList>) -> Option<Box<ExpressionList>> {
    l.map(|el| {
        Box::new(ExpressionList {
            expression: el
                .expression
                .as_ref()
                .map(|e| Box::new(clone_expression_node(e.as_ref()))),
            next: clone_expression_list(el.next.as_deref()),
        })
    })
}

fn clone_literal_node(l: &LiteralNode) -> LiteralNode {
    LiteralNode {
        type_: l.type_,
        value: match &l.value {
            LiteralValue::IntValue(i) => LiteralValue::IntValue(*i),
            LiteralValue::BooleanValue(b) => LiteralValue::BooleanValue(*b),
            LiteralValue::StringValue(s) => LiteralValue::StringValue(s.clone()),
        },
    }
}

// =================================================
//   tokenizer
// =================================================

pub fn match_token(state: &mut ParseState, kind: TokenKind) -> i32 {
    if let Some(ref t) = state.token {
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
    let len = bytes.len();

    // Skip whitespaces
    while (state.pos as usize) < len && is_space_char(bytes[state.pos as usize]) {
        state.pos += 1;
    }

    let pos = state.pos as usize;

    let mut new_token = Token {
        kind: TokenKind::Eof,
        next: None,
        val: 0,
        str: String::new(),
    };

    if pos >= len {
        new_token.kind = TokenKind::Eof;
        new_token.str = String::new();
    } else {
        let c = bytes[pos];
        if c == b'(' {
            new_token.kind = TokenKind::LParen;
            new_token.str = "(".to_string();
            state.pos += 1;
        } else if c == b')' {
            new_token.kind = TokenKind::RParen;
            new_token.str = ")".to_string();
            state.pos += 1;
        } else if c == b'\'' {
            new_token.kind = TokenKind::Quote;
            new_token.str = "'".to_string();
            state.pos += 1;
        } else if is_alpha_char(c) || is_op_char(c) {
            // tokenize symbol
            let start = pos;
            while (state.pos as usize) < len {
                let cc = bytes[state.pos as usize];
                if is_alnum_char(cc) || is_op_char(cc) {
                    state.pos += 1;
                } else {
                    break;
                }
            }
            let s = std::str::from_utf8(&bytes[start..(state.pos as usize)])
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
        } else if is_digit_char(c) {
            // tokenize digit
            let start = pos;
            while (state.pos as usize) < len && is_digit_char(bytes[state.pos as usize]) {
                state.pos += 1;
            }
            let s = std::str::from_utf8(&bytes[start..(state.pos as usize)]).unwrap_or("");
            let val: i32 = s.parse().unwrap_or(0);
            new_token.kind = TokenKind::Digit;
            new_token.val = val;
        } else if c == b'"' {
            // tokenize string
            state.pos += 1; // skip opening "
            let start = state.pos as usize;
            while (state.pos as usize) < len && bytes[state.pos as usize] != b'"' {
                state.pos += 1;
            }
            let s = std::str::from_utf8(&bytes[start..(state.pos as usize)])
                .unwrap_or("")
                .to_string();
            new_token.kind = TokenKind::String;
            new_token.str = s;
            if (state.pos as usize) < len && bytes[state.pos as usize] == b'"' {
                state.pos += 1;
            }
        } else if c == b';' {
            // tokenize comment - skip to end of line
            while (state.pos as usize) < len && bytes[state.pos as usize] != b'\n' {
                state.pos += 1;
            }
            return next(source, state);
        } else {
            panic!("Unexpected token: {}", c as char);
        }
    }

    state.token = Some(Box::new(new_token));
}

// =================================================
//   parser
// =================================================

fn append_to_program(program: &mut ProgramNode, expr: ExpressionNode) {
    let new_item = Box::new(ExpressionList {
        expression: Some(Box::new(expr)),
        next: None,
    });

    if program.expressions.is_none() {
        program.expressions = Some(new_item);
        return;
    }
    let mut current = program.expressions.as_mut().unwrap();
    while current.next.is_some() {
        current = current.next.as_mut().unwrap();
    }
    current.next = Some(new_item);
}

fn append_to_list_node(list_node: &mut ListNode, expr: ExpressionNode) {
    let new_item = Box::new(ExpressionList {
        expression: Some(Box::new(expr)),
        next: None,
    });

    if list_node.expressions.is_none() {
        list_node.expressions = Some(new_item);
        return;
    }
    let mut current = list_node.expressions.as_mut().unwrap();
    while current.next.is_some() {
        current = current.next.as_mut().unwrap();
    }
    current.next = Some(new_item);
}

fn append_to_symbolic_exp(sym: &mut SymbolicExpNode, expr: ExpressionNode) {
    let new_item = Box::new(ExpressionList {
        expression: Some(Box::new(expr)),
        next: None,
    });

    if sym.expressions.is_none() {
        sym.expressions = Some(new_item);
        return;
    }
    let mut current = sym.expressions.as_mut().unwrap();
    while current.next.is_some() {
        current = current.next.as_mut().unwrap();
    }
    current.next = Some(new_item);
}

fn parse_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    match current_kind(state) {
        TokenKind::LParen => parse_symbolic_expression(source, state),
        TokenKind::Quote => parse_list_expression(source, state),
        TokenKind::Symbol => parse_symbol_expression(source, state),
        TokenKind::Digit | TokenKind::String | TokenKind::True | TokenKind::False => {
            parse_literal_expression(source, state)
        }
        kind => {
            let s = state
                .token
                .as_ref()
                .map(|t| t.str.clone())
                .unwrap_or_default();
            panic!("Unexpected token: kind={:?} str={}", kind, s);
        }
    }
}

fn parse_symbolic_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    let mut sym_exp = SymbolicExpNode { expressions: None };
    next(source, state); // eat (
    while current_kind(state) != TokenKind::RParen {
        let expr = parse_expression(source, state);
        append_to_symbolic_exp(&mut sym_exp, expr);
    }
    next(source, state); // eat )
    ExpressionNode {
        type_: ExpressionType::SymbolicExp,
        data: ExpressionData::SymbolicExp(Some(Box::new(sym_exp))),
    }
}

fn parse_list_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    let mut list_node = ListNode { expressions: None };
    next(source, state); // eat '
    next(source, state); // eat (
    while current_kind(state) != TokenKind::RParen {
        let expr = parse_expression(source, state);
        append_to_list_node(&mut list_node, expr);
    }
    next(source, state); // eat )
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
        data: ExpressionData::Symbol(Some(Box::new(SymbolNode {
            symbol_name: name,
        }))),
    }
}

fn parse_literal_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    let kind = current_kind(state);
    let lit_node = match kind {
        TokenKind::Digit => {
            let val = state.token.as_ref().map(|t| t.val).unwrap_or(0);
            LiteralNode {
                type_: LiteralType::Integer,
                value: LiteralValue::IntValue(val),
            }
        }
        TokenKind::String => {
            let s = state
                .token
                .as_ref()
                .map(|t| t.str.clone())
                .unwrap_or_default();
            LiteralNode {
                type_: LiteralType::String,
                value: LiteralValue::StringValue(s),
            }
        }
        TokenKind::True => LiteralNode {
            type_: LiteralType::Boolean,
            value: LiteralValue::BooleanValue(true),
        },
        TokenKind::False => LiteralNode {
            type_: LiteralType::Boolean,
            value: LiteralValue::BooleanValue(false),
        },
        _ => panic!("unexpected literal token kind"),
    };
    next(source, state);
    ExpressionNode {
        type_: ExpressionType::Literal,
        data: ExpressionData::Literal(Some(Box::new(lit_node))),
    }
}

fn parse_program(source: &str, state: &mut ParseState, result: &mut ParseResult) {
    next(source, state);
    let mut program = ProgramNode { expressions: None };
    while current_kind(state) != TokenKind::Eof {
        let expr = parse_expression(source, state);
        append_to_program(&mut program, expr);
    }
    result.program = Some(Box::new(program));
}

pub fn parse(source: &str, state: &mut ParseState, result: &mut ParseResult) {
    parse_program(source, state, result);
}

// =================================================
//   environment helpers
// =================================================

fn env_lookup(env: &Env, name: &str) -> Option<Object> {
    for i in 0..MAX_BINDINGS {
        if env.bindings[i].symbol_name.is_empty() {
            break;
        }
        if env.bindings[i].symbol_name == name {
            if let Some(ref v) = env.bindings[i].value {
                return Some(clone_object(v.as_ref()));
            }
        }
    }
    if let Some(ref parent) = env.parent {
        env_lookup(parent.as_ref(), name)
    } else {
        None
    }
}

fn env_set(env: &mut Env, name: &str, obj: Object) {
    // first check if the name already exists
    for i in 0..MAX_BINDINGS {
        if env.bindings[i].symbol_name.is_empty() {
            // not found, add new binding here
            env.bindings[i].symbol_name = name.to_string();
            env.bindings[i].value = Some(Box::new(obj));
            return;
        }
        if env.bindings[i].symbol_name == name {
            env.bindings[i].value = Some(Box::new(obj));
            return;
        }
    }
}

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

fn obj_int(v: i32) -> Object {
    Object {
        marked: false,
        type_: ObjectType::Integer,
        value: ObjectValue::IntValue(v),
    }
}

fn obj_string(s: String) -> Object {
    Object {
        marked: false,
        type_: ObjectType::String,
        value: ObjectValue::StringValue(s),
    }
}

fn obj_bool(v: bool) -> Object {
    Object {
        marked: false,
        type_: ObjectType::Bool,
        value: ObjectValue::BoolValue(if v { 1 } else { 0 }),
    }
}

fn obj_nil() -> Object {
    make_nil_object()
}

fn obj_int_value(o: &Object) -> i32 {
    if let ObjectValue::IntValue(v) = &o.value {
        *v
    } else {
        0
    }
}

fn obj_string_value(o: &Object) -> &str {
    if let ObjectValue::StringValue(s) = &o.value {
        s.as_str()
    } else {
        ""
    }
}

fn obj_bool_value(o: &Object) -> i32 {
    if let ObjectValue::BoolValue(v) = &o.value {
        *v
    } else {
        0
    }
}

fn is_last_cons_cell(c: &ConsCell) -> bool {
    if let Some(ref cdr) = c.cdr {
        matches!(cdr.type_, ObjectType::Nil)
    } else {
        true
    }
}

fn build_list_from_objects(objects: Vec<Object>) -> Object {
    if objects.is_empty() {
        return obj_nil();
    }
    // Build from end to start
    let mut iter = objects.into_iter().rev();
    let last = iter.next().unwrap();
    let mut tail_cons = ConsCell {
        type_: ConsCellType::Nil,
        car: Some(Box::new(last)),
        cdr: Some(Box::new(obj_nil())),
    };
    for obj in iter {
        let new_cdr = Object {
            marked: false,
            type_: ObjectType::List,
            value: ObjectValue::ListValue(Some(Box::new(tail_cons))),
        };
        tail_cons = ConsCell {
            type_: ConsCellType::Cell,
            car: Some(Box::new(obj)),
            cdr: Some(Box::new(new_cdr)),
        };
    }
    Object {
        marked: false,
        type_: ObjectType::List,
        value: ObjectValue::ListValue(Some(Box::new(tail_cons))),
    }
}

fn collect_expression_list(list: Option<&ExpressionList>) -> Vec<&ExpressionNode> {
    let mut result = Vec::new();
    let mut current = list;
    while let Some(node) = current {
        if let Some(ref e) = node.expression {
            result.push(e.as_ref());
        }
        current = node.next.as_deref();
    }
    result
}

// =================================================
//   evaluator
// =================================================

pub fn evaluate_expression(
    expression: &ExpressionNode,
    result: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
    match expression.type_ {
        ExpressionType::Literal => evaluate_literal_expression(expression, result),
        ExpressionType::Symbol => evaluate_symbol_expression(expression, result, env),
        ExpressionType::List => evaluate_list_expression(expression, result, env, context),
        ExpressionType::SymbolicExp => {
            evaluate_symbolic_expression(expression, result, env, context)
        }
    }
}

fn evaluate_literal_expression(expression: &ExpressionNode, result: &mut Object) {
    if let ExpressionData::Literal(Some(ref lit)) = expression.data {
        match &lit.value {
            LiteralValue::IntValue(v) => {
                result.type_ = ObjectType::Integer;
                result.value = ObjectValue::IntValue(*v);
            }
            LiteralValue::StringValue(s) => {
                result.type_ = ObjectType::String;
                result.value = ObjectValue::StringValue(s.clone());
            }
            LiteralValue::BooleanValue(b) => {
                result.type_ = ObjectType::Bool;
                result.value = ObjectValue::BoolValue(if *b { 1 } else { 0 });
            }
        }
    }
}

fn evaluate_symbol_expression(
    expression: &ExpressionNode,
    result: &mut Object,
    env: &mut Env,
) {
    if let ExpressionData::Symbol(Some(ref sym)) = expression.data {
        let name = sym.symbol_name.as_str();
        if name == "nil" {
            result.type_ = ObjectType::Nil;
            result.value = ObjectValue::IntValue(0);
            return;
        }
        if let Some(val) = env_lookup(env, name) {
            *result = val;
        } else {
            panic!("Undefined symbol: {}", name);
        }
    }
}

fn evaluate_list_expression(
    expression: &ExpressionNode,
    result: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
    let exprs_list: Option<&ExpressionList> = if let ExpressionData::List(Some(ref l)) =
        expression.data
    {
        l.expressions.as_deref()
    } else {
        None
    };

    let exprs = collect_expression_list(exprs_list);
    if exprs.is_empty() {
        result.type_ = ObjectType::Nil;
        result.value = ObjectValue::IntValue(0);
        return;
    }

    let mut evaluated_items: Vec<Object> = Vec::with_capacity(exprs.len());
    for e in exprs {
        let mut item = make_nil_object();
        evaluate_expression(e, &mut item, env, context);
        evaluated_items.push(item);
    }

    let list_obj = build_list_from_objects(evaluated_items);
    *result = list_obj;
}

fn get_symbolic_exprs(expression: &ExpressionNode) -> Vec<&ExpressionNode> {
    if let ExpressionData::SymbolicExp(Some(ref s)) = expression.data {
        collect_expression_list(s.expressions.as_deref())
    } else {
        Vec::new()
    }
}

fn evaluate_symbolic_expression(
    expression: &ExpressionNode,
    result: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
    let exprs = get_symbolic_exprs(expression);
    if exprs.is_empty() {
        result.type_ = ObjectType::Nil;
        result.value = ObjectValue::IntValue(0);
        return;
    }

    let head = exprs[0];
    let head_name: String = match &head.data {
        ExpressionData::Symbol(Some(s)) => s.symbol_name.clone(),
        _ => {
            panic!("S-exp must be started with symbol.");
        }
    };

    let args: Vec<&ExpressionNode> = exprs[1..].to_vec();

    match head_name.as_str() {
        "if" => {
            if args.is_empty() {
                panic!("if must have condition.");
            }
            let mut cond_obj = make_nil_object();
            evaluate_expression(args[0], &mut cond_obj, env, context);
            if bool_val(&cond_obj) {
                if args.len() < 2 {
                    panic!("if must have then clause.");
                }
                evaluate_expression(args[1], result, env, context);
            } else if args.len() >= 3 {
                evaluate_expression(args[2], result, env, context);
            } else {
                result.type_ = ObjectType::Nil;
                result.value = ObjectValue::IntValue(0);
            }
        }
        "while" => {
            if args.len() < 2 {
                panic!("while must have condition and body.");
            }
            loop {
                let mut cond_obj = make_nil_object();
                evaluate_expression(args[0], &mut cond_obj, env, context);
                if bool_val(&cond_obj) {
                    let mut tmp = make_nil_object();
                    evaluate_expression(args[1], &mut tmp, env, context);
                    *result = tmp;
                } else {
                    result.type_ = ObjectType::Nil;
                    result.value = ObjectValue::IntValue(0);
                    break;
                }
            }
        }
        "=" => {
            if args.len() < 2 {
                panic!("assignment must have name and value.");
            }
            let name: String = match &args[0].data {
                ExpressionData::Symbol(Some(s)) => s.symbol_name.clone(),
                _ => panic!("Variable name must be symbol."),
            };
            let mut val = make_nil_object();
            evaluate_expression(args[1], &mut val, env, context);
            *result = clone_object(&val);
            env_set(env, &name, val);
        }
        "defun" => {
            if args.len() < 3 {
                panic!("defun must have name, params, body.");
            }
            let name: String = match &args[0].data {
                ExpressionData::Symbol(Some(s)) => s.symbol_name.clone(),
                _ => panic!("Function name must be symbol."),
            };
            let params_exprs: Vec<&ExpressionNode> = match &args[1].data {
                ExpressionData::SymbolicExp(Some(s)) => {
                    collect_expression_list(s.expressions.as_deref())
                }
                _ => panic!("Function parameter must be list."),
            };
            let mut param_names: Vec<String> = Vec::new();
            for p in params_exprs {
                match &p.data {
                    ExpressionData::Symbol(Some(s)) => param_names.push(s.symbol_name.clone()),
                    _ => panic!("Function parameter must be symbol."),
                }
            }
            let body_clone = clone_expression_node(args[2]);
            let function = Function {
                param_symbol_names: param_names,
                body: Some(Box::new(body_clone)),
            };
            result.type_ = ObjectType::Function;
            result.value = ObjectValue::FunctionValue(Some(Box::new(function)));
            let to_store = clone_object(result);
            env_set(env, &name, to_store);
        }
        "+" => {
            let (op1, op2) = eval_two_args(&args, env, context);
            defined_function_add(&op1, &op2, result);
        }
        "-" => {
            let (op1, op2) = eval_two_args(&args, env, context);
            defined_function_sub(&op1, &op2, result);
        }
        "*" => {
            let (op1, op2) = eval_two_args(&args, env, context);
            defined_function_mul(&op1, &op2, result);
        }
        "/" => {
            let (op1, op2) = eval_two_args(&args, env, context);
            defined_function_div(&op1, &op2, result);
        }
        "%" => {
            let (op1, op2) = eval_two_args(&args, env, context);
            defined_function_mod(&op1, &op2, result);
        }
        "||" => {
            for a in &args {
                let mut o = make_nil_object();
                evaluate_expression(a, &mut o, env, context);
                if bool_val(&o) {
                    result.type_ = ObjectType::Bool;
                    result.value = ObjectValue::BoolValue(1);
                    return;
                }
            }
            result.type_ = ObjectType::Bool;
            result.value = ObjectValue::BoolValue(0);
        }
        "&&" => {
            for a in &args {
                let mut o = make_nil_object();
                evaluate_expression(a, &mut o, env, context);
                if !bool_val(&o) {
                    result.type_ = ObjectType::Bool;
                    result.value = ObjectValue::BoolValue(0);
                    return;
                }
            }
            result.type_ = ObjectType::Bool;
            result.value = ObjectValue::BoolValue(1);
        }
        "<" => {
            let (op1, op2) = eval_two_args(&args, env, context);
            defined_function_lt(&op1, &op2, result);
        }
        ">" => {
            let (op1, op2) = eval_two_args(&args, env, context);
            defined_function_gt(&op1, &op2, result);
        }
        "eq" => {
            let (op1, op2) = eval_two_args(&args, env, context);
            *result = obj_bool(eq(&op1, &op2));
        }
        "not" => {
            if args.is_empty() {
                panic!("not requires 1 argument");
            }
            let mut o = make_nil_object();
            evaluate_expression(args[0], &mut o, env, context);
            defined_function_not(&o, result);
        }
        "print" => {
            if args.is_empty() {
                panic!("print requires 1 argument");
            }
            let mut o = make_nil_object();
            evaluate_expression(args[0], &mut o, env, context);
            let s = stringify_object(&o);
            println!("{}", s);
            result.type_ = ObjectType::Nil;
            result.value = ObjectValue::IntValue(0);
        }
        "car" => {
            if args.is_empty() {
                panic!("car requires 1 argument");
            }
            let mut o = make_nil_object();
            evaluate_expression(args[0], &mut o, env, context);
            defined_function_car(&o, result);
        }
        "cdr" => {
            if args.is_empty() {
                panic!("cdr requires 1 argument");
            }
            let mut o = make_nil_object();
            evaluate_expression(args[0], &mut o, env, context);
            defined_function_cdr(&o, result);
        }
        "cons" => {
            let (op1, op2) = eval_two_args(&args, env, context);
            defined_function_cons(op1, op2, result);
        }
        "split" => {
            let (op1, op2) = eval_two_args(&args, env, context);
            defined_function_split(&op1, &op2, result);
        }
        "list-ref" => {
            let (op1, op2) = eval_two_args(&args, env, context);
            defined_function_list_ref(&op1, &op2, result);
        }
        "progn" => {
            let mut last: Option<Object> = None;
            for a in &args {
                let mut o = make_nil_object();
                evaluate_expression(a, &mut o, env, context);
                last = Some(o);
            }
            *result = last.unwrap_or_else(make_nil_object);
        }
        "remove-whitespaces" => {
            if args.is_empty() {
                panic!("remove-whitespaces requires 1 argument");
            }
            let mut o = make_nil_object();
            evaluate_expression(args[0], &mut o, env, context);
            defined_function_remove_whitespaces(&o, result);
        }
        "pop" => {
            if args.is_empty() {
                panic!("pop requires 1 argument");
            }
            let mut o = make_nil_object();
            evaluate_expression(args[0], &mut o, env, context);
            defined_function_pop(&o, result);
        }
        "push" => {
            // Note: in C, the order is reversed in some places. We follow the
            // C semantics where push retrieves the list arg first.
            if args.len() < 2 {
                panic!("push requires 2 arguments");
            }
            let mut list_obj = make_nil_object();
            evaluate_expression(args[0], &mut list_obj, env, context);
            let mut val_obj = make_nil_object();
            evaluate_expression(args[1], &mut val_obj, env, context);
            defined_function_push(list_obj, val_obj, result);
        }
        "length" => {
            if args.is_empty() {
                panic!("length requires 1 argument");
            }
            let mut o = make_nil_object();
            evaluate_expression(args[0], &mut o, env, context);
            defined_function_length(&o, result);
        }
        "is-int-string" => {
            if args.is_empty() {
                panic!("is-int-string requires 1 argument");
            }
            let mut o = make_nil_object();
            evaluate_expression(args[0], &mut o, env, context);
            defined_function_is_int_string(&o, result);
        }
        "parse-int" => {
            if args.is_empty() {
                panic!("parse-int requires 1 argument");
            }
            let mut o = make_nil_object();
            evaluate_expression(args[0], &mut o, env, context);
            defined_function_parse_int(&o, result);
        }
        "string-ref" => {
            let (op1, op2) = eval_two_args(&args, env, context);
            defined_function_string_ref(&op1, &op2, result);
        }
        _ => {
            // user-defined function call
            if let Some(found_obj) = env_lookup(env, &head_name) {
                if let ObjectValue::FunctionValue(Some(func)) = &found_obj.value {
                    let param_names = func.param_symbol_names.clone();
                    let body = func.body.as_ref().map(|b| clone_expression_node(b.as_ref()));
                    // evaluate arguments in current env
                    let mut evaluated_args: Vec<Object> = Vec::new();
                    for (i, _) in param_names.iter().enumerate() {
                        if i < args.len() {
                            let mut o = make_nil_object();
                            evaluate_expression(args[i], &mut o, env, context);
                            evaluated_args.push(o);
                        } else {
                            evaluated_args.push(make_nil_object());
                        }
                    }
                    // build new env that copies current env's bindings (flat closure)
                    let mut new_env = Env {
                        bindings: std::array::from_fn(|_| Binding {
                            symbol_name: String::new(),
                            value: None,
                        }),
                        parent: None,
                    };
                    // copy bindings from current env
                    for i in 0..MAX_BINDINGS {
                        if env.bindings[i].symbol_name.is_empty() {
                            break;
                        }
                        new_env.bindings[i].symbol_name = env.bindings[i].symbol_name.clone();
                        new_env.bindings[i].value = env.bindings[i]
                            .value
                            .as_ref()
                            .map(|v| Box::new(clone_object(v.as_ref())));
                    }
                    // bind parameters
                    for (name, val) in param_names.iter().zip(evaluated_args.into_iter()) {
                        env_set(&mut new_env, name, val);
                    }
                    if let Some(body_expr) = body {
                        evaluate_expression(&body_expr, result, &mut new_env, context);
                    } else {
                        result.type_ = ObjectType::Nil;
                        result.value = ObjectValue::IntValue(0);
                    }
                    return;
                }
            }
            panic!("Undefined function: {}", head_name);
        }
    }
}

fn eval_two_args(
    args: &[&ExpressionNode],
    env: &mut Env,
    context: &mut AllocatorContext,
) -> (Object, Object) {
    if args.len() < 2 {
        panic!("Function requires 2 arguments");
    }
    let mut a = make_nil_object();
    let mut b = make_nil_object();
    evaluate_expression(args[0], &mut a, env, context);
    evaluate_expression(args[1], &mut b, env, context);
    (a, b)
}

fn eq(op1: &Object, op2: &Object) -> bool {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => obj_int_value(op1) == obj_int_value(op2),
        (ObjectType::String, ObjectType::String) => obj_string_value(op1) == obj_string_value(op2),
        (ObjectType::Bool, ObjectType::Bool) => obj_bool_value(op1) == obj_bool_value(op2),
        (ObjectType::Nil, ObjectType::Nil) => true,
        (ObjectType::List, ObjectType::List) => {
            // C compares pointers; we approximate with structural equality
            // Both should be considered equal if both contain the same content.
            // For simplicity, compare shapes.
            list_eq(op1, op2)
        }
        _ => false,
    }
}

fn list_eq(a: &Object, b: &Object) -> bool {
    match (&a.value, &b.value) {
        (ObjectValue::ListValue(av), ObjectValue::ListValue(bv)) => match (av, bv) {
            (None, None) => true,
            (None, _) | (_, None) => false,
            (Some(ac), Some(bc)) => cons_eq(ac.as_ref(), bc.as_ref()),
        },
        _ => false,
    }
}

fn cons_eq(a: &ConsCell, b: &ConsCell) -> bool {
    let car_eq = match (&a.car, &b.car) {
        (Some(x), Some(y)) => eq(x.as_ref(), y.as_ref()),
        (None, None) => true,
        _ => false,
    };
    if !car_eq {
        return false;
    }
    match (&a.cdr, &b.cdr) {
        (Some(x), Some(y)) => eq(x.as_ref(), y.as_ref()),
        (None, None) => true,
        _ => false,
    }
}

fn defined_function_add(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(obj_int_value(op1) + obj_int_value(op2));
        }
        (ObjectType::String, ObjectType::String) => {
            let mut s = obj_string_value(op1).to_string();
            s.push_str(obj_string_value(op2));
            evaluated.type_ = ObjectType::String;
            evaluated.value = ObjectValue::StringValue(s);
        }
        _ => panic!("Type error: operands for + must be integers or strings."),
    }
}

fn defined_function_sub(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if let (ObjectType::Integer, ObjectType::Integer) = (op1.type_, op2.type_) {
        evaluated.type_ = ObjectType::Integer;
        evaluated.value = ObjectValue::IntValue(obj_int_value(op1) - obj_int_value(op2));
    } else {
        panic!("Type error: operands for - must be integers.");
    }
}

fn defined_function_mul(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if let (ObjectType::Integer, ObjectType::Integer) = (op1.type_, op2.type_) {
        evaluated.type_ = ObjectType::Integer;
        evaluated.value = ObjectValue::IntValue(obj_int_value(op1) * obj_int_value(op2));
    } else {
        panic!("Type error: operands for * must be integers.");
    }
}

fn defined_function_div(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if let (ObjectType::Integer, ObjectType::Integer) = (op1.type_, op2.type_) {
        evaluated.type_ = ObjectType::Integer;
        evaluated.value = ObjectValue::IntValue(obj_int_value(op1) / obj_int_value(op2));
    } else {
        panic!("Type error: operands for / must be integers.");
    }
}

fn defined_function_mod(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if let (ObjectType::Integer, ObjectType::Integer) = (op1.type_, op2.type_) {
        evaluated.type_ = ObjectType::Integer;
        evaluated.value = ObjectValue::IntValue(obj_int_value(op1) % obj_int_value(op2));
    } else {
        panic!("Type error: operands for % must be integers.");
    }
}

fn defined_function_lt(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if let (ObjectType::Integer, ObjectType::Integer) = (op1.type_, op2.type_) {
        *evaluated = obj_bool(obj_int_value(op1) < obj_int_value(op2));
    } else {
        panic!("Type error: operands for < must be integers.");
    }
}

fn defined_function_gt(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if let (ObjectType::Integer, ObjectType::Integer) = (op1.type_, op2.type_) {
        *evaluated = obj_bool(obj_int_value(op1) > obj_int_value(op2));
    } else {
        panic!("Type error: operands for > must be integers.");
    }
}

fn defined_function_not(op: &Object, evaluated: &mut Object) {
    if let ObjectType::Bool = op.type_ {
        *evaluated = obj_bool(obj_bool_value(op) == 0);
    } else {
        panic!("Type error: not operand must be boolean.");
    }
}

fn defined_function_car(op: &Object, evaluated: &mut Object) {
    if !matches!(op.type_, ObjectType::List) {
        panic!("Type error: car operand must be list.");
    }
    if let ObjectValue::ListValue(Some(ref cc)) = op.value {
        if let Some(ref car) = cc.car {
            *evaluated = clone_object(car.as_ref());
            return;
        }
    }
    *evaluated = obj_nil();
}

fn defined_function_cdr(op: &Object, evaluated: &mut Object) {
    if !matches!(op.type_, ObjectType::List) {
        panic!("Type error: cdr operand must be list.");
    }
    if let ObjectValue::ListValue(Some(ref cc)) = op.value {
        if let Some(ref cdr) = cc.cdr {
            *evaluated = clone_object(cdr.as_ref());
            return;
        }
    }
    *evaluated = obj_nil();
}

fn defined_function_cons(op1: Object, op2: Object, evaluated: &mut Object) {
    let car_box = Box::new(op1);
    match op2.type_ {
        ObjectType::List => {
            let cc = ConsCell {
                type_: ConsCellType::Cell,
                car: Some(car_box),
                cdr: Some(Box::new(op2)),
            };
            evaluated.type_ = ObjectType::List;
            evaluated.value = ObjectValue::ListValue(Some(Box::new(cc)));
        }
        ObjectType::Nil => {
            let cc = ConsCell {
                type_: ConsCellType::Nil,
                car: Some(car_box),
                cdr: Some(Box::new(op2)),
            };
            evaluated.type_ = ObjectType::List;
            evaluated.value = ObjectValue::ListValue(Some(Box::new(cc)));
        }
        _ => {
            // wrap op2 into a list
            let inner_cc = ConsCell {
                type_: ConsCellType::Nil,
                car: Some(Box::new(op2)),
                cdr: Some(Box::new(obj_nil())),
            };
            let inner_obj = Object {
                marked: false,
                type_: ObjectType::List,
                value: ObjectValue::ListValue(Some(Box::new(inner_cc))),
            };
            let cc = ConsCell {
                type_: ConsCellType::Cell,
                car: Some(car_box),
                cdr: Some(Box::new(inner_obj)),
            };
            evaluated.type_ = ObjectType::List;
            evaluated.value = ObjectValue::ListValue(Some(Box::new(cc)));
        }
    }
}

fn defined_function_split(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if !matches!(op1.type_, ObjectType::String) {
        panic!("Type error: split first operand must be string.");
    }
    if !matches!(op2.type_, ObjectType::String) {
        panic!("Type error: split second operand must be string.");
    }
    let s1 = obj_string_value(op1);
    let s2 = obj_string_value(op2);

    let parts: Vec<Object> = if s2.is_empty() {
        s1.chars().map(|c| obj_string(c.to_string())).collect()
    } else {
        // C uses strtok which uses each char in delim as separator, and
        // skips empty tokens
        let delims: Vec<char> = s2.chars().collect();
        let mut tokens: Vec<String> = Vec::new();
        let mut current = String::new();
        for c in s1.chars() {
            if delims.contains(&c) {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            } else {
                current.push(c);
            }
        }
        if !current.is_empty() {
            tokens.push(current);
        }
        tokens.into_iter().map(obj_string).collect()
    };

    *evaluated = build_list_from_objects(parts);
}

fn defined_function_list_ref(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if !matches!(op1.type_, ObjectType::List) {
        panic!("Type error: list-ref first operand must be list.");
    }
    if !matches!(op2.type_, ObjectType::Integer) {
        panic!("Type error: list-ref second operand must be integer.");
    }
    let index = obj_int_value(op2);
    let mut current_cell = match &op1.value {
        ObjectValue::ListValue(Some(c)) => c.as_ref(),
        _ => panic!("Index out of range."),
    };
    for _ in 0..index {
        let cdr = current_cell.cdr.as_ref().expect("Index out of range.");
        if matches!(cdr.type_, ObjectType::Nil) {
            panic!("Index out of range.");
        }
        current_cell = match &cdr.value {
            ObjectValue::ListValue(Some(c)) => c.as_ref(),
            _ => panic!("Index out of range."),
        };
    }
    if let Some(ref car) = current_cell.car {
        *evaluated = clone_object(car.as_ref());
    } else {
        *evaluated = obj_nil();
    }
}

fn defined_function_remove_whitespaces(op: &Object, evaluated: &mut Object) {
    if !matches!(op.type_, ObjectType::String) {
        panic!("Type error: remove-whitespaces operand must be string.");
    }
    let s = obj_string_value(op);
    let result: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    *evaluated = obj_string(result);
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
    // walk to the last cons cell, return its car
    let mut last_car: Option<Object> = None;
    let mut current = match &op.value {
        ObjectValue::ListValue(Some(c)) => Some(c.as_ref()),
        _ => None,
    };
    while let Some(cc) = current {
        if is_last_cons_cell(cc) {
            last_car = cc.car.as_ref().map(|o| clone_object(o.as_ref()));
            break;
        }
        let cdr = cc.cdr.as_deref();
        current = match cdr {
            Some(o) => match &o.value {
                ObjectValue::ListValue(Some(c)) => Some(c.as_ref()),
                _ => None,
            },
            None => None,
        };
    }
    *evaluated = last_car.unwrap_or_else(obj_nil);
}

fn defined_function_push(list_obj: Object, val_obj: Object, evaluated: &mut Object) {
    // Build a new list = original list + [val_obj]
    let mut items: Vec<Object> = Vec::new();
    let mut current = match &list_obj.value {
        ObjectValue::ListValue(Some(c)) => Some(clone_conscell(c.as_ref())),
        _ => None,
    };
    while let Some(cc) = current {
        let is_last = is_last_cons_cell(&cc);
        if let Some(car) = cc.car {
            items.push(*car);
        }
        if is_last {
            break;
        }
        let cdr = cc.cdr;
        current = match cdr {
            Some(o) => match o.value {
                ObjectValue::ListValue(Some(c)) => Some(*c),
                _ => None,
            },
            None => None,
        };
    }
    items.push(val_obj);
    *evaluated = build_list_from_objects(items);
}

fn defined_function_length(op: &Object, evaluated: &mut Object) {
    match op.type_ {
        ObjectType::Nil => {
            *evaluated = obj_int(0);
        }
        ObjectType::List => {
            let mut len = 0i32;
            let mut current = match &op.value {
                ObjectValue::ListValue(Some(c)) => Some(c.as_ref()),
                _ => None,
            };
            while let Some(cc) = current {
                len += 1;
                if is_last_cons_cell(cc) {
                    break;
                }
                let cdr = cc.cdr.as_deref();
                current = match cdr {
                    Some(o) => match &o.value {
                        ObjectValue::ListValue(Some(c)) => Some(c.as_ref()),
                        _ => None,
                    },
                    None => None,
                };
            }
            *evaluated = obj_int(len);
        }
        ObjectType::String => {
            let s = obj_string_value(op);
            *evaluated = obj_int(s.len() as i32);
        }
        _ => panic!("Type error: length operand must be list or string."),
    }
}

fn defined_function_is_int_string(op: &Object, evaluated: &mut Object) {
    if let ObjectType::String = op.type_ {
        let s = obj_string_value(op);
        let all_digits = !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
        *evaluated = obj_bool(all_digits);
    } else {
        *evaluated = obj_bool(false);
    }
}

fn defined_function_parse_int(op: &Object, evaluated: &mut Object) {
    if !matches!(op.type_, ObjectType::String) {
        panic!("Type error: parse-int operand must be string.");
    }
    let s = obj_string_value(op);
    let v: i32 = s.parse().unwrap_or(0);
    *evaluated = obj_int(v);
}

fn defined_function_string_ref(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if !matches!(op1.type_, ObjectType::String) {
        panic!("Type error: string-ref first operand must be string.");
    }
    if !matches!(op2.type_, ObjectType::Integer) {
        panic!("Type error: string-ref second operand must be integer.");
    }
    let s = obj_string_value(op1);
    let idx = obj_int_value(op2);
    if idx < 0 || (idx as usize) >= s.len() {
        panic!("Index out of range.");
    }
    let ch = s.chars().nth(idx as usize).unwrap();
    *evaluated = obj_string(ch.to_string());
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
    if let Some(ref program) = result.program {
        let mut env = Env {
            bindings: std::array::from_fn(|_| Binding {
                symbol_name: String::new(),
                value: None,
            }),
            parent: None,
        };
        let mut context = init_allocator();
        let mut current = program.expressions.as_deref();
        while let Some(node) = current {
            if let Some(ref e) = node.expression {
                let mut evaluated = make_nil_object();
                evaluate_expression(e.as_ref(), &mut evaluated, &mut env, &mut context);
            }
            current = node.next.as_deref();
        }
    }
}

pub fn stringify_object(obj: &Object) -> String {
    match obj.type_ {
        ObjectType::Integer => obj_int_value(obj).to_string(),
        ObjectType::String => obj_string_value(obj).to_string(),
        ObjectType::Bool => {
            if obj_bool_value(obj) != 0 {
                "T".to_string()
            } else {
                "F".to_string()
            }
        }
        ObjectType::List => {
            let mut s = String::from("(");
            let mut current = match &obj.value {
                ObjectValue::ListValue(Some(c)) => Some(c.as_ref()),
                _ => None,
            };
            let mut first = true;
            while let Some(cc) = current {
                if !first {
                    s.push(' ');
                }
                first = false;
                if let Some(ref car) = cc.car {
                    s.push_str(&stringify_object(car.as_ref()));
                }
                if is_last_cons_cell(cc) {
                    break;
                }
                let cdr = cc.cdr.as_deref();
                current = match cdr {
                    Some(o) => match &o.value {
                        ObjectValue::ListValue(Some(c)) => Some(c.as_ref()),
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

pub fn init_env(env: &mut Env) {
    env.parent = None;
    for i in 0..MAX_BINDINGS {
        env.bindings[i].symbol_name = String::new();
        env.bindings[i].value = None;
    }
}

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
    Some(Box::new(make_nil_object()))
}
