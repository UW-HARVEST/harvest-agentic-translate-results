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
fn empty_bindings() -> [Binding; MAX_BINDINGS] {
    std::array::from_fn(|_| Binding {
        symbol_name: String::new(),
        value: None,
    })
}
fn nil_object() -> Object {
    Object {
        marked: false,
        type_: ObjectType::Nil,
        value: ObjectValue::IntValue(0),
    }
}
fn int_object(value: i32) -> Object {
    Object {
        marked: false,
        type_: ObjectType::Integer,
        value: ObjectValue::IntValue(value),
    }
}
fn string_object(value: String) -> Object {
    Object {
        marked: false,
        type_: ObjectType::String,
        value: ObjectValue::StringValue(value),
    }
}
fn bool_object(value: bool) -> Object {
    Object {
        marked: false,
        type_: ObjectType::Bool,
        value: ObjectValue::BoolValue(if value { 1 } else { 0 }),
    }
}
fn list_object(cell: ConsCell) -> Object {
    Object {
        marked: false,
        type_: ObjectType::List,
        value: ObjectValue::ListValue(Some(Box::new(cell))),
    }
}
fn function_object(function: Function) -> Object {
    Object {
        marked: false,
        type_: ObjectType::Function,
        value: ObjectValue::FunctionValue(Some(Box::new(function))),
    }
}
fn object_bool_value(obj: &Object) -> bool {
    match obj.type_ {
        ObjectType::Bool => matches!(obj.value, ObjectValue::BoolValue(v) if v != 0),
        ObjectType::Nil => false,
        _ => true,
    }
}
fn object_string_value(obj: &Object) -> Option<&str> {
    match &obj.value {
        ObjectValue::StringValue(value) if matches!(obj.type_, ObjectType::String) => Some(value),
        _ => None,
    }
}
fn object_int_value(obj: &Object) -> Option<i32> {
    match obj.value {
        ObjectValue::IntValue(value) if matches!(obj.type_, ObjectType::Integer) => Some(value),
        _ => None,
    }
}
fn object_list_value(obj: &Object) -> Option<&ConsCell> {
    match &obj.value {
        ObjectValue::ListValue(Some(cell)) if matches!(obj.type_, ObjectType::List) => Some(cell),
        _ => None,
    }
}
fn object_list_value_mut(obj: &mut Object) -> Option<&mut ConsCell> {
    match &mut obj.value {
        ObjectValue::ListValue(Some(cell)) if matches!(obj.type_, ObjectType::List) => Some(cell),
        _ => None,
    }
}
fn object_function_value(obj: &Object) -> Option<&Function> {
    match &obj.value {
        ObjectValue::FunctionValue(Some(function))
            if matches!(obj.type_, ObjectType::Function) =>
        {
            Some(function)
        }
        _ => None,
    }
}
fn clone_literal_value(value: &LiteralValue) -> LiteralValue {
    match value {
        LiteralValue::IntValue(v) => LiteralValue::IntValue(*v),
        LiteralValue::BooleanValue(v) => LiteralValue::BooleanValue(*v),
        LiteralValue::StringValue(v) => LiteralValue::StringValue(v.clone()),
    }
}
fn clone_literal_node(node: &LiteralNode) -> LiteralNode {
    LiteralNode {
        type_: node.type_,
        value: clone_literal_value(&node.value),
    }
}
fn clone_symbol_node(node: &SymbolNode) -> SymbolNode {
    SymbolNode {
        symbol_name: node.symbol_name.clone(),
    }
}
fn clone_expression_list(list: &Option<Box<ExpressionList>>) -> Option<Box<ExpressionList>> {
    list.as_ref().map(|node| {
        Box::new(ExpressionList {
            expression: node
                .expression
                .as_ref()
                .map(|expr| Box::new(clone_expression_node(expr))),
            next: clone_expression_list(&node.next),
        })
    })
}
fn clone_expression_node(node: &ExpressionNode) -> ExpressionNode {
    ExpressionNode {
        type_: node.type_,
        data: match &node.data {
            ExpressionData::SymbolicExp(value) => ExpressionData::SymbolicExp(value.as_ref().map(
                |symbolic| {
                    Box::new(SymbolicExpNode {
                        expressions: clone_expression_list(&symbolic.expressions),
                    })
                },
            )),
            ExpressionData::List(value) => ExpressionData::List(value.as_ref().map(|list| {
                Box::new(ListNode {
                    expressions: clone_expression_list(&list.expressions),
                })
            })),
            ExpressionData::Literal(value) => ExpressionData::Literal(
                value.as_ref().map(|literal| Box::new(clone_literal_node(literal))),
            ),
            ExpressionData::Symbol(value) => {
                ExpressionData::Symbol(value.as_ref().map(|symbol| Box::new(clone_symbol_node(symbol))))
            }
        },
    }
}
fn clone_function(function: &Function) -> Function {
    Function {
        param_symbol_names: function.param_symbol_names.clone(),
        body: function
            .body
            .as_ref()
            .map(|body| Box::new(clone_expression_node(body))),
    }
}
fn clone_object(obj: &Object) -> Object {
    Object {
        marked: obj.marked,
        type_: obj.type_,
        value: match &obj.value {
            ObjectValue::IntValue(v) => ObjectValue::IntValue(*v),
            ObjectValue::StringValue(v) => ObjectValue::StringValue(v.clone()),
            ObjectValue::BoolValue(v) => ObjectValue::BoolValue(*v),
            ObjectValue::ListValue(value) => {
                ObjectValue::ListValue(value.as_ref().map(|cell| Box::new(clone_cons_cell(cell))))
            }
            ObjectValue::FunctionValue(value) => ObjectValue::FunctionValue(
                value
                    .as_ref()
                    .map(|function| Box::new(clone_function(function))),
            ),
        },
    }
}
fn clone_cons_cell(cell: &ConsCell) -> ConsCell {
    ConsCell {
        type_: cell.type_,
        car: cell.car.as_ref().map(|car| Box::new(clone_object(car))),
        cdr: cell.cdr.as_ref().map(|cdr| Box::new(clone_object(cdr))),
    }
}
fn clone_env(env: &Env) -> Env {
    let mut bindings = empty_bindings();
    for (index, binding) in env.bindings.iter().enumerate() {
        bindings[index] = Binding {
            symbol_name: binding.symbol_name.clone(),
            value: binding.value.as_ref().map(|value| Box::new(clone_object(value))),
        };
    }
    Env {
        bindings,
        parent: env.parent.as_ref().map(|parent| Box::new(clone_env(parent))),
    }
}
fn token_at<'a>(state: &'a ParseState) -> &'a Token {
    state
        .token
        .as_deref()
        .unwrap_or_else(|| panic!("No current token"))
}
fn is_op(ch: char) -> bool {
    matches!(ch, '+' | '-' | '*' | '/' | '%' | '|' | '&' | '=' | '<' | '>')
}
fn source_char_at(source: &str, pos: usize) -> Option<char> {
    source.as_bytes().get(pos).map(|byte| *byte as char)
}
fn expression_vec_to_list(expressions: Vec<ExpressionNode>) -> Option<Box<ExpressionList>> {
    let mut list = None;
    for expression in expressions.into_iter().rev() {
        list = Some(Box::new(ExpressionList {
            expression: Some(Box::new(expression)),
            next: list,
        }));
    }
    list
}
fn parse_expression_node(source: &str, state: &mut ParseState) -> ExpressionNode {
    if match_token(state, TokenKind::LParen) == 1 {
        next(source, state);
        let mut expressions = Vec::new();
        while match_token(state, TokenKind::RParen) == 0 {
            expressions.push(parse_expression_node(source, state));
        }
        next(source, state);
        ExpressionNode {
            type_: ExpressionType::SymbolicExp,
            data: ExpressionData::SymbolicExp(Some(Box::new(SymbolicExpNode {
                expressions: expression_vec_to_list(expressions),
            }))),
        }
    } else if match_token(state, TokenKind::Quote) == 1 {
        next(source, state);
        if match_token(state, TokenKind::LParen) == 0 {
            panic!("Expected '(' after quote");
        }
        next(source, state);
        let mut expressions = Vec::new();
        while match_token(state, TokenKind::RParen) == 0 {
            expressions.push(parse_expression_node(source, state));
        }
        next(source, state);
        ExpressionNode {
            type_: ExpressionType::List,
            data: ExpressionData::List(Some(Box::new(ListNode {
                expressions: expression_vec_to_list(expressions),
            }))),
        }
    } else if match_token(state, TokenKind::Symbol) == 1 {
        let symbol_name = token_at(state).str.clone();
        next(source, state);
        ExpressionNode {
            type_: ExpressionType::Symbol,
            data: ExpressionData::Symbol(Some(Box::new(SymbolNode { symbol_name }))),
        }
    } else if match_token(state, TokenKind::Digit) == 1 {
        let value = token_at(state).val;
        next(source, state);
        ExpressionNode {
            type_: ExpressionType::Literal,
            data: ExpressionData::Literal(Some(Box::new(LiteralNode {
                type_: LiteralType::Integer,
                value: LiteralValue::IntValue(value),
            }))),
        }
    } else if match_token(state, TokenKind::String) == 1 {
        let value = token_at(state).str.clone();
        next(source, state);
        ExpressionNode {
            type_: ExpressionType::Literal,
            data: ExpressionData::Literal(Some(Box::new(LiteralNode {
                type_: LiteralType::String,
                value: LiteralValue::StringValue(value),
            }))),
        }
    } else if match_token(state, TokenKind::True) == 1 {
        next(source, state);
        ExpressionNode {
            type_: ExpressionType::Literal,
            data: ExpressionData::Literal(Some(Box::new(LiteralNode {
                type_: LiteralType::Boolean,
                value: LiteralValue::BooleanValue(true),
            }))),
        }
    } else if match_token(state, TokenKind::False) == 1 {
        next(source, state);
        ExpressionNode {
            type_: ExpressionType::Literal,
            data: ExpressionData::Literal(Some(Box::new(LiteralNode {
                type_: LiteralType::Boolean,
                value: LiteralValue::BooleanValue(false),
            }))),
        }
    } else {
        panic!("Unexpected token while parsing");
    }
}
fn expression_refs(list: &Option<Box<ExpressionList>>) -> Vec<&ExpressionNode> {
    let mut refs = Vec::new();
    let mut current = list.as_deref();
    while let Some(node) = current {
        if let Some(expression) = node.expression.as_deref() {
            refs.push(expression);
        }
        current = node.next.as_deref();
    }
    refs
}
fn list_elements(obj: &Object) -> Vec<&Object> {
    let mut items = Vec::new();
    let mut current = object_list_value(obj);
    while let Some(cell) = current {
        if let Some(car) = cell.car.as_deref() {
            items.push(car);
        }
        let next = cell.cdr.as_deref();
        if matches!(next.map(|obj| obj.type_), Some(ObjectType::List)) {
            current = next.and_then(object_list_value);
        } else {
            break;
        }
    }
    items
}
fn list_length(obj: &Object) -> usize {
    list_elements(obj).len()
}
fn build_list_from_objects(objects: Vec<Object>) -> Object {
    if objects.is_empty() {
        return nil_object();
    }
    let mut tail = nil_object();
    for object in objects.into_iter().rev() {
        tail = list_object(ConsCell {
            type_: ConsCellType::Cell,
            car: Some(Box::new(object)),
            cdr: Some(Box::new(tail)),
        });
    }
    tail
}
fn object_eq(left: &Object, right: &Object) -> bool {
    if std::mem::discriminant(&left.type_) != std::mem::discriminant(&right.type_) {
        return false;
    }
    match left.type_ {
        ObjectType::Integer => object_int_value(left) == object_int_value(right),
        ObjectType::String => object_string_value(left) == object_string_value(right),
        ObjectType::Bool => match (&left.value, &right.value) {
            (ObjectValue::BoolValue(a), ObjectValue::BoolValue(b)) => a == b,
            _ => false,
        },
        ObjectType::List => match (&left.value, &right.value) {
            (ObjectValue::ListValue(Some(a)), ObjectValue::ListValue(Some(b))) => {
                std::ptr::eq(a.as_ref(), b.as_ref())
            }
            _ => false,
        },
        ObjectType::Nil => true,
        ObjectType::Function => false,
    }
}
fn lookup_symbol<'a>(env: &'a Env, symbol: &str) -> Option<&'a Object> {
    for binding in &env.bindings {
        if binding.symbol_name.is_empty() {
            break;
        }
        if binding.symbol_name == symbol {
            return binding.value.as_deref();
        }
    }
    env.parent
        .as_deref()
        .and_then(|parent| lookup_symbol(parent, symbol))
}
fn lookup_symbol_mut<'a>(env: &'a mut Env, symbol: &str) -> Option<&'a mut Object> {
    for index in 0..MAX_BINDINGS {
        if env.bindings[index].symbol_name.is_empty() {
            break;
        }
        if env.bindings[index].symbol_name == symbol {
            return env.bindings[index].value.as_deref_mut();
        }
    }
    env.parent
        .as_deref_mut()
        .and_then(|parent| lookup_symbol_mut(parent, symbol))
}
fn set_object_to_env(env: &mut Env, symbol_name: &str, obj: Object) {
    for binding in &mut env.bindings {
        if binding.symbol_name.is_empty() {
            binding.symbol_name = symbol_name.to_string();
            binding.value = Some(Box::new(obj));
            return;
        }
        if binding.symbol_name == symbol_name {
            binding.value = Some(Box::new(obj));
            return;
        }
    }
    panic!("Environment binding limit exceeded");
}
fn eval_to_object(
    expression: &ExpressionNode,
    env: &mut Env,
    context: &mut AllocatorContext,
) -> Object {
    let mut result = nil_object();
    evaluate_expression(expression, &mut result, env, context);
    result
}
fn append_to_list(list: &mut Object, value: Object) {
    let mut current = match object_list_value_mut(list) {
        Some(cell) => cell,
        None => panic!("Type error: push second operand must be list."),
    };
    loop {
        let is_last = current
            .cdr
            .as_deref()
            .is_none_or(|cdr| matches!(cdr.type_, ObjectType::Nil));
        if is_last {
            current.cdr = Some(Box::new(list_object(ConsCell {
                type_: ConsCellType::Cell,
                car: Some(Box::new(value)),
                cdr: Some(Box::new(nil_object())),
            })));
            break;
        }
        current = current
            .cdr
            .as_deref_mut()
            .and_then(object_list_value_mut)
            .unwrap_or_else(|| panic!("Malformed list"));
    }
}
fn pop_from_list(list: &mut Object) -> Object {
    let mut elements: Vec<Object> = list_elements(list)
        .into_iter()
        .map(clone_object)
        .collect();
    let last = elements.pop().unwrap_or_else(nil_object);
    if elements.len() > 1 {
        *list = build_list_from_objects(elements);
    }
    last
}
fn write_result(result: &mut Object, value: Object) {
    *result = value;
}
pub fn match_token(state: &mut ParseState, kind: TokenKind) -> i32 {
    match state.token.as_deref() {
        Some(token) if token.kind == kind => 1,
        _ => 0,
    }
}
pub fn next(source: &str, state: &mut ParseState) {
    loop {
        while let Some(ch) = source_char_at(source, state.pos as usize) {
            if ch.is_whitespace() {
                state.pos += 1;
            } else {
                break;
            }
        }
        if source_char_at(source, state.pos as usize) == Some(';') {
            while let Some(ch) = source_char_at(source, state.pos as usize) {
                state.pos += 1;
                if ch == '\n' {
                    break;
                }
            }
            continue;
        }
        break;
    }

    let pos = state.pos as usize;
    let token = match source_char_at(source, pos) {
        Some('(') => {
            state.pos += 1;
            Token {
                kind: TokenKind::LParen,
                next: None,
                val: 0,
                str: "(".to_string(),
            }
        }
        Some(')') => {
            state.pos += 1;
            Token {
                kind: TokenKind::RParen,
                next: None,
                val: 0,
                str: ")".to_string(),
            }
        }
        Some('\'') => {
            state.pos += 1;
            Token {
                kind: TokenKind::Quote,
                next: None,
                val: 0,
                str: "'".to_string(),
            }
        }
        None | Some('\0') => Token {
            kind: TokenKind::Eof,
            next: None,
            val: 0,
            str: "\0".to_string(),
        },
        Some(ch) if ch.is_ascii_alphabetic() || is_op(ch) => {
            let start = state.pos as usize;
            while let Some(ch) = source_char_at(source, state.pos as usize) {
                if ch.is_ascii_alphanumeric() || is_op(ch) {
                    state.pos += 1;
                } else {
                    break;
                }
            }
            let text = source[start..state.pos as usize].to_string();
            let kind = match text.as_str() {
                "true" => TokenKind::True,
                "false" => TokenKind::False,
                _ => TokenKind::Symbol,
            };
            Token {
                kind,
                next: None,
                val: 0,
                str: text,
            }
        }
        Some(ch) if ch.is_ascii_digit() => {
            let start = state.pos as usize;
            while let Some(ch) = source_char_at(source, state.pos as usize) {
                if ch.is_ascii_digit() {
                    state.pos += 1;
                } else {
                    break;
                }
            }
            let text = &source[start..state.pos as usize];
            Token {
                kind: TokenKind::Digit,
                next: None,
                val: text.parse::<i32>().unwrap_or(0),
                str: text.to_string(),
            }
        }
        Some('"') => {
            state.pos += 1;
            let start = state.pos as usize;
            while let Some(ch) = source_char_at(source, state.pos as usize) {
                if ch == '"' || ch == '\0' {
                    break;
                }
                state.pos += 1;
            }
            let text = source[start..state.pos as usize].to_string();
            if source_char_at(source, state.pos as usize) == Some('"') {
                state.pos += 1;
            }
            Token {
                kind: TokenKind::String,
                next: None,
                val: 0,
                str: text,
            }
        }
        Some(ch) => panic!("Unexpected token: {ch}"),
    };

    state.token = Some(Box::new(token));
}
pub fn parse(source: &str, state: &mut ParseState, result: &mut ParseResult) {
    next(source, state);
    let mut expressions = Vec::new();
    while match_token(state, TokenKind::Eof) == 0 {
        expressions.push(parse_expression_node(source, state));
    }
    result.program = Some(Box::new(ProgramNode {
        expressions: expression_vec_to_list(expressions),
    }));
}
pub fn evaluate_expression(
expression: &ExpressionNode,
result: &mut Object,
env: &mut Env,
context: &mut AllocatorContext,
) {
    match expression.type_ {
        ExpressionType::Literal => {
            let literal = match &expression.data {
                ExpressionData::Literal(Some(literal)) => literal.as_ref(),
                _ => panic!("Malformed literal expression"),
            };
            let value = match &literal.value {
                LiteralValue::IntValue(value) => int_object(*value),
                LiteralValue::BooleanValue(value) => bool_object(*value),
                LiteralValue::StringValue(value) => string_object(value.clone()),
            };
            write_result(result, value);
        }
        ExpressionType::Symbol => {
            let symbol = match &expression.data {
                ExpressionData::Symbol(Some(symbol)) => symbol.symbol_name.as_str(),
                _ => panic!("Malformed symbol expression"),
            };
            if symbol == "nil" {
                write_result(result, nil_object());
            } else if let Some(value) = lookup_symbol(env, symbol) {
                write_result(result, clone_object(value));
            } else {
                panic!("Undefined symbol: {symbol}");
            }
        }
        ExpressionType::List => {
            let list = match &expression.data {
                ExpressionData::List(Some(list)) => list,
                _ => panic!("Malformed list expression"),
            };
            let exprs = expression_refs(&list.expressions);
            let values = exprs
                .into_iter()
                .map(|expr| eval_to_object(expr, env, context))
                .collect::<Vec<_>>();
            write_result(result, build_list_from_objects(values));
        }
        ExpressionType::SymbolicExp => {
            let symbolic = match &expression.data {
                ExpressionData::SymbolicExp(Some(symbolic)) => symbolic,
                _ => panic!("Malformed symbolic expression"),
            };
            let exprs = expression_refs(&symbolic.expressions);
            if exprs.is_empty() {
                write_result(result, nil_object());
                return;
            }
            let head = exprs[0];
            let symbol = match &head.data {
                ExpressionData::Symbol(Some(symbol)) => symbol.symbol_name.as_str(),
                _ => panic!("S-exp must be started with symbol."),
            };
            match symbol {
                "if" => {
                    let cond = exprs.get(1).copied().unwrap_or_else(|| panic!("if must have condition."));
                    let then_expr = exprs.get(2).copied().unwrap_or_else(|| panic!("if must have then clause."));
                    let cond_obj = eval_to_object(cond, env, context);
                    if object_bool_value(&cond_obj) {
                        evaluate_expression(then_expr, result, env, context);
                    } else if let Some(else_expr) = exprs.get(3).copied() {
                        evaluate_expression(else_expr, result, env, context);
                    } else {
                        write_result(result, nil_object());
                    }
                }
                "while" => {
                    let cond = exprs.get(1).copied().unwrap_or_else(|| panic!("while must have condition."));
                    if exprs.len() < 3 {
                        panic!("while must have body.");
                    }
                    loop {
                        let cond_obj = eval_to_object(cond, env, context);
                        if !object_bool_value(&cond_obj) {
                            write_result(result, nil_object());
                            break;
                        }
                        for body_expr in exprs.iter().skip(2) {
                            evaluate_expression(body_expr, result, env, context);
                        }
                    }
                }
                "=" => {
                    let symbol_name = match exprs.get(1).and_then(|expr| match &expr.data {
                        ExpressionData::Symbol(Some(symbol)) => Some(symbol.symbol_name.clone()),
                        _ => None,
                    }) {
                        Some(symbol) => symbol,
                        None => panic!("Variable name must be symbol."),
                    };
                    let value_expr = exprs
                        .get(2)
                        .copied()
                        .unwrap_or_else(|| panic!("assignment must have expression."));
                    let value = eval_to_object(value_expr, env, context);
                    set_object_to_env(env, &symbol_name, clone_object(&value));
                    write_result(result, value);
                }
                "defun" => {
                    let symbol_name = match exprs.get(1).and_then(|expr| match &expr.data {
                        ExpressionData::Symbol(Some(symbol)) => Some(symbol.symbol_name.clone()),
                        _ => None,
                    }) {
                        Some(symbol) => symbol,
                        None => panic!("Function name must be symbol."),
                    };
                    let params_expr = exprs
                        .get(2)
                        .copied()
                        .unwrap_or_else(|| panic!("Function must have parameter."));
                    let param_symbol_names = match &params_expr.data {
                        ExpressionData::SymbolicExp(Some(node)) => expression_refs(&node.expressions)
                            .into_iter()
                            .map(|expr| match &expr.data {
                                ExpressionData::Symbol(Some(symbol)) => symbol.symbol_name.clone(),
                                _ => panic!("Function parameter must be symbol."),
                            })
                            .collect::<Vec<_>>(),
                        _ => panic!("Function parameter must be list."),
                    };
                    let body = exprs
                        .get(3)
                        .copied()
                        .unwrap_or_else(|| panic!("Function must have body."));
                    let value = function_object(Function {
                        param_symbol_names,
                        body: Some(Box::new(clone_expression_node(body))),
                    });
                    set_object_to_env(env, &symbol_name, clone_object(&value));
                    write_result(result, value);
                }
                "+" => {
                    let left = eval_to_object(exprs[1], env, context);
                    let right = eval_to_object(exprs[2], env, context);
                    let value = match (left.type_, right.type_) {
                        (ObjectType::Integer, ObjectType::Integer) => {
                            int_object(object_int_value(&left).unwrap() + object_int_value(&right).unwrap())
                        }
                        (ObjectType::String, ObjectType::String) => string_object(format!(
                            "{}{}",
                            object_string_value(&left).unwrap(),
                            object_string_value(&right).unwrap()
                        )),
                        _ => panic!("Type error: operands for + must be integers or strings."),
                    };
                    write_result(result, value);
                }
                "-" => {
                    let left = eval_to_object(exprs[1], env, context);
                    let right = eval_to_object(exprs[2], env, context);
                    let value = int_object(
                        object_int_value(&left).unwrap_or_else(|| panic!("Type error: operands for - must be integers."))
                            - object_int_value(&right).unwrap_or_else(|| panic!("Type error: operands for - must be integers.")),
                    );
                    write_result(result, value);
                }
                "*" => {
                    let left = eval_to_object(exprs[1], env, context);
                    let right = eval_to_object(exprs[2], env, context);
                    let value = int_object(
                        object_int_value(&left).unwrap_or_else(|| panic!("Type error: operands for * must be integers."))
                            * object_int_value(&right).unwrap_or_else(|| panic!("Type error: operands for * must be integers.")),
                    );
                    write_result(result, value);
                }
                "/" => {
                    let left = eval_to_object(exprs[1], env, context);
                    let right = eval_to_object(exprs[2], env, context);
                    let value = int_object(
                        object_int_value(&left).unwrap_or_else(|| panic!("Type error: operands for / must be integers."))
                            / object_int_value(&right).unwrap_or_else(|| panic!("Type error: operands for / must be integers.")),
                    );
                    write_result(result, value);
                }
                "%" => {
                    let left = eval_to_object(exprs[1], env, context);
                    let right = eval_to_object(exprs[2], env, context);
                    let value = int_object(
                        object_int_value(&left).unwrap_or_else(|| panic!("Type error: operands for % must be integers."))
                            % object_int_value(&right).unwrap_or_else(|| panic!("Type error: operands for % must be integers.")),
                    );
                    write_result(result, value);
                }
                "||" => {
                    let value = exprs
                        .iter()
                        .skip(1)
                        .map(|expr| eval_to_object(expr, env, context))
                        .any(|obj| object_bool_value(&obj));
                    write_result(result, bool_object(value));
                }
                "&&" => {
                    let value = exprs
                        .iter()
                        .skip(1)
                        .map(|expr| eval_to_object(expr, env, context))
                        .all(|obj| object_bool_value(&obj));
                    write_result(result, bool_object(value));
                }
                "<" => {
                    let left = eval_to_object(exprs[1], env, context);
                    let right = eval_to_object(exprs[2], env, context);
                    write_result(
                        result,
                        bool_object(
                            object_int_value(&left).unwrap_or_else(|| panic!("Type error: operands for < must be integers."))
                                < object_int_value(&right).unwrap_or_else(|| panic!("Type error: operands for < must be integers.")),
                        ),
                    );
                }
                ">" => {
                    let left = eval_to_object(exprs[1], env, context);
                    let right = eval_to_object(exprs[2], env, context);
                    write_result(
                        result,
                        bool_object(
                            object_int_value(&left).unwrap_or_else(|| panic!("Type error: operands for > must be integers."))
                                > object_int_value(&right).unwrap_or_else(|| panic!("Type error: operands for > must be integers.")),
                        ),
                    );
                }
                "eq" => {
                    let left = eval_to_object(exprs[1], env, context);
                    let right = eval_to_object(exprs[2], env, context);
                    write_result(result, bool_object(object_eq(&left, &right)));
                }
                "not" => {
                    let value = eval_to_object(exprs[1], env, context);
                    match value.value {
                        ObjectValue::BoolValue(v) if matches!(value.type_, ObjectType::Bool) => {
                            write_result(result, bool_object(v == 0))
                        }
                        _ => panic!("Type error: not operand must be boolean."),
                    }
                }
                "print" => {
                    let value = eval_to_object(exprs[1], env, context);
                    println!("{}", stringify_object(&value));
                    write_result(result, nil_object());
                }
                "car" => {
                    let value = eval_to_object(exprs[1], env, context);
                    let cell = object_list_value(&value)
                        .unwrap_or_else(|| panic!("Type error: car operand must be list."));
                    let car = cell.car.as_deref().map(clone_object).unwrap_or_else(nil_object);
                    write_result(result, car);
                }
                "cdr" => {
                    let value = eval_to_object(exprs[1], env, context);
                    let cell = object_list_value(&value)
                        .unwrap_or_else(|| panic!("Type error: cdr operand must be list."));
                    let cdr = cell.cdr.as_deref().map(clone_object).unwrap_or_else(nil_object);
                    write_result(result, cdr);
                }
                "cons" => {
                    let left = eval_to_object(exprs[1], env, context);
                    let right = eval_to_object(exprs[2], env, context);
                    let cdr = if matches!(right.type_, ObjectType::List | ObjectType::Nil) {
                        right
                    } else {
                        build_list_from_objects(vec![right])
                    };
                    write_result(
                        result,
                        list_object(ConsCell {
                            type_: ConsCellType::Cell,
                            car: Some(Box::new(left)),
                            cdr: Some(Box::new(cdr)),
                        }),
                    );
                }
                "split" => {
                    let source_value = eval_to_object(exprs[1], env, context);
                    let delim_value = eval_to_object(exprs[2], env, context);
                    let source = object_string_value(&source_value)
                        .unwrap_or_else(|| panic!("Type error: split first operand must be string."));
                    let delim = object_string_value(&delim_value)
                        .unwrap_or_else(|| panic!("Type error: split second operand must be string."));
                    let parts = if delim.is_empty() {
                        source
                            .bytes()
                            .map(|byte| string_object((byte as char).to_string()))
                            .collect::<Vec<_>>()
                    } else {
                        source
                            .split(|ch| delim.contains(ch))
                            .filter(|part| !part.is_empty())
                            .map(|part| string_object(part.to_string()))
                            .collect::<Vec<_>>()
                    };
                    write_result(result, build_list_from_objects(parts));
                }
                "list-ref" => {
                    let list = eval_to_object(exprs[1], env, context);
                    let index = eval_to_object(exprs[2], env, context);
                    let index = object_int_value(&index)
                        .unwrap_or_else(|| panic!("Type error: list-ref second operand must be integer."));
                    let items = list_elements(&list);
                    let value = items
                        .get(index as usize)
                        .map(|obj| clone_object(obj))
                        .unwrap_or_else(|| panic!("Index out of range."));
                    write_result(result, value);
                }
                "progn" => {
                    let mut value = nil_object();
                    for expr in exprs.iter().skip(1) {
                        evaluate_expression(expr, &mut value, env, context);
                    }
                    write_result(result, value);
                }
                "remove-whitespaces" => {
                    let value = eval_to_object(exprs[1], env, context);
                    let source = object_string_value(&value)
                        .unwrap_or_else(|| panic!("Type error: remove-whitespaces operand must be string."));
                    write_result(
                        result,
                        string_object(source.chars().filter(|ch| !ch.is_whitespace()).collect()),
                    );
                }
                "pop" => {
                    if let ExpressionData::Symbol(Some(symbol_node)) = &exprs[1].data {
                        if let Some(bound) = lookup_symbol_mut(env, &symbol_node.symbol_name) {
                            if matches!(bound.type_, ObjectType::Nil) {
                                write_result(result, nil_object());
                            } else if !matches!(bound.type_, ObjectType::List) {
                                panic!("Type error: pop operand must be list.");
                            } else {
                                write_result(result, pop_from_list(bound));
                            }
                            return;
                        }
                    }
                    let mut value = eval_to_object(exprs[1], env, context);
                    if matches!(value.type_, ObjectType::Nil) {
                        write_result(result, nil_object());
                    } else if !matches!(value.type_, ObjectType::List) {
                        panic!("Type error: pop operand must be list.");
                    } else {
                        write_result(result, pop_from_list(&mut value));
                    }
                }
                "push" => {
                    let pushed = eval_to_object(exprs[1], env, context);
                    if let ExpressionData::Symbol(Some(symbol_node)) = &exprs[2].data {
                        if let Some(bound) = lookup_symbol_mut(env, &symbol_node.symbol_name) {
                            match bound.type_ {
                                ObjectType::Nil => {
                                    *bound = build_list_from_objects(vec![clone_object(&pushed)]);
                                }
                                ObjectType::List => append_to_list(bound, clone_object(&pushed)),
                                _ => panic!("Type error: push second operand must be list."),
                            }
                            write_result(result, pushed);
                            return;
                        }
                    }
                    let mut list = eval_to_object(exprs[2], env, context);
                    match list.type_ {
                        ObjectType::Nil => {}
                        ObjectType::List => append_to_list(&mut list, clone_object(&pushed)),
                        _ => panic!("Type error: push second operand must be list."),
                    }
                    write_result(result, pushed);
                }
                "length" => {
                    let value = eval_to_object(exprs[1], env, context);
                    let length = match value.type_ {
                        ObjectType::Nil => 0,
                        ObjectType::List => list_length(&value) as i32,
                        ObjectType::String => object_string_value(&value).unwrap().len() as i32,
                        _ => panic!("Type error: length operand must be list or string."),
                    };
                    write_result(result, int_object(length));
                }
                "is-int-string" => {
                    let value = eval_to_object(exprs[1], env, context);
                    let matches = object_string_value(&value)
                        .is_some_and(|text| text.chars().all(|ch| ch.is_ascii_digit()));
                    write_result(result, bool_object(matches));
                }
                "parse-int" => {
                    let value = eval_to_object(exprs[1], env, context);
                    let text = object_string_value(&value)
                        .unwrap_or_else(|| panic!("Type error: parse-int operand must be string."));
                    if !text.chars().all(|ch| ch.is_ascii_digit()) {
                        panic!("Type error: parse-int operand must be string of digits.");
                    }
                    write_result(result, int_object(text.parse::<i32>().unwrap_or(0)));
                }
                "string-ref" => {
                    let source_value = eval_to_object(exprs[1], env, context);
                    let index_value = eval_to_object(exprs[2], env, context);
                    let source = object_string_value(&source_value)
                        .unwrap_or_else(|| panic!("Type error: string-ref first operand must be string."));
                    let index = object_int_value(&index_value)
                        .unwrap_or_else(|| panic!("Type error: string-ref second operand must be integer."));
                    let byte = source
                        .as_bytes()
                        .get(index as usize)
                        .copied()
                        .unwrap_or_else(|| panic!("Index out of range."));
                    write_result(result, string_object((byte as char).to_string()));
                }
                _ => {
                    let function_object = lookup_symbol(env, symbol)
                        .map(clone_object)
                        .unwrap_or_else(|| panic!("Undefined function: {symbol}"));
                    let function = object_function_value(&function_object)
                        .unwrap_or_else(|| panic!("Undefined function: {symbol}"));
                    let mut new_env = Env {
                        bindings: empty_bindings(),
                        parent: Some(Box::new(clone_env(env))),
                    };
                    if function.param_symbol_names.len() > exprs.len().saturating_sub(1) {
                        panic!("Function parameter count mismatch.");
                    }
                    for (param_name, arg_expr) in function
                        .param_symbol_names
                        .iter()
                        .zip(exprs.iter().skip(1))
                    {
                        let value = eval_to_object(arg_expr, env, context);
                        set_object_to_env(&mut new_env, param_name, value);
                    }
                    let body = function.body.as_deref().unwrap_or_else(|| panic!("Function must have body."));
                    evaluate_expression(body, result, &mut new_env, context);
                }
            }
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
    let mut env = Env {
        bindings: empty_bindings(),
        parent: None,
    };
    init_env(&mut env);
    let mut context = init_allocator();
    if let Some(program) = result.program.as_deref() {
        let mut current = program.expressions.as_deref();
        while let Some(node) = current {
            if let Some(expression) = node.expression.as_deref() {
                let mut value = nil_object();
                evaluate_expression(expression, &mut value, &mut env, &mut context);
            }
            current = node.next.as_deref();
        }
    }
}
pub fn stringify_object(obj: &Object) -> String {
    match obj.type_ {
        ObjectType::Integer => object_int_value(obj).unwrap_or(0).to_string(),
        ObjectType::String => object_string_value(obj).unwrap_or("").to_string(),
        ObjectType::Bool => {
            if object_bool_value(obj) {
                "T".to_string()
            } else {
                "F".to_string()
            }
        }
        ObjectType::List => {
            let parts = list_elements(obj)
                .into_iter()
                .map(stringify_object)
                .collect::<Vec<_>>();
            format!("({})", parts.join(" "))
        }
        ObjectType::Nil => "nil".to_string(),
        ObjectType::Function => "<function>".to_string(),
    }
}
pub fn init_env(env: &mut Env) {
    env.parent = None;
    env.bindings = empty_bindings();
}
pub fn init_allocator() -> AllocatorContext {
    AllocatorContext {
        gc_less_mode: 0,
        stack: Some(Box::new(ObjectStack {
            objects: std::array::from_fn(|_| None),
            top: -1,
        })),
        memory_pool: None,
        free_bitmap: [0; FREE_BITMAP_SIZE],
    }
}
pub fn allocate(context: &mut AllocatorContext, env: &mut Env) -> Option<Box<Object>> {
    let _ = context;
    let _ = env;
    Some(Box::new(nil_object()))
}
