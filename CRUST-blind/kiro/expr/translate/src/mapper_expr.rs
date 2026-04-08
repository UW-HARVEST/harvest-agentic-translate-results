use std::f32::consts::PI;
use std::collections::HashMap;
use std::sync::Arc;
use lazy_static::lazy_static;
#[derive(Clone, Copy, Debug)]
pub enum MapperSignalValue {
    F(f32),
    I32(i32),
}
impl MapperSignalValue {
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            MapperSignalValue::F(f) => Some(*f),
            _ => None,
        }
    }
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            MapperSignalValue::I32(i) => Some(*i),
            _ => None,
        }
    }
}
const STACK_SIZE: usize = 256;
const TRACING: bool = false;
macro_rules! trace {
    ($($arg:tt)*) => {
        if TRACING {
            println!("-- {}", format!($($arg)*));
        }
    };
}
macro_rules! die_unless {
    ($cond:expr, $($arg:tt)*) => {
        if !$cond {
            println!("-- {}", format!($($arg)*));
            assert!($cond);
        }
    };
}
fn minf(x: f32, y: f32) -> f32 {
    if y < x { y } else { x }
}
fn maxf(x: f32, y: f32) -> f32 {
    if y > x { y } else { x }
}
fn pif() -> f32 {
    PI
}
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExprFunc {
    Unknown = -1,
    Pow = 0,
    Sin,
    Cos,
    Tan,
    Abs,
    Sqrt,
    Log,
    Log10,
    Exp,
    Floor,
    Round,
    Ceil,
    Asin,
    Acos,
    Atan,
    Atan2,
    Sinh,
    Cosh,
    Tanh,
    Logb,
    Exp2,
    Log2,
    Hypot,
    Cbrt,
    Trunc,
    Min,
    Max,
    Pi,
    NFuncs,
}
#[derive(Debug, Clone, Copy)]
struct FunctionEntry {
    name: &'static str,
    arity: u32,
    func: fn(f32, f32) -> f32,
}
lazy_static::lazy_static! {
    static ref FUNCTION_TABLE: HashMap<&'static str, FunctionEntry> = {
        let mut m = HashMap::new();
        m.insert("pow", FunctionEntry { name: "pow", arity: 2, func: f32::powf });
        m.insert("sin", FunctionEntry { name: "sin", arity: 1, func: |x, _| x.sin() });
        m.insert("cos", FunctionEntry { name: "cos", arity: 1, func: |x, _| x.cos() });
        m.insert("tan", FunctionEntry { name: "tan", arity: 1, func: |x, _| x.tan() });
        m.insert("abs", FunctionEntry { name: "abs", arity: 1, func: |x, _| x.abs() });
        m.insert("sqrt", FunctionEntry { name: "sqrt", arity: 1, func: |x, _| x.sqrt() });
        m.insert("log", FunctionEntry { name: "log", arity: 1, func: |x, _| x.ln() });
        m.insert("log10", FunctionEntry { name: "log10", arity: 1, func: |x, _| x.log10() });
        m.insert("exp", FunctionEntry { name: "exp", arity: 1, func: |x, _| x.exp() });
        m.insert("floor", FunctionEntry { name: "floor", arity: 1, func: |x, _| x.floor() });
        m.insert("round", FunctionEntry { name: "round", arity: 1, func: |x, _| x.round() });
        m.insert("ceil", FunctionEntry { name: "ceil", arity: 1, func: |x, _| x.ceil() });
        m.insert("min", FunctionEntry { name: "min", arity: 2, func: minf });
        m.insert("max", FunctionEntry { name: "max", arity: 2, func: maxf });
        m.insert("pi", FunctionEntry { name: "pi", arity: 0, func: |_, _| pif() });
        m
    };
}

// Extended function table covering all C functions
static FUNC_TABLE_ARRAY: &[(ExprFunc, &str, u32)] = &[
    (ExprFunc::Pow, "pow", 2),
    (ExprFunc::Sin, "sin", 1),
    (ExprFunc::Cos, "cos", 1),
    (ExprFunc::Tan, "tan", 1),
    (ExprFunc::Abs, "abs", 1),
    (ExprFunc::Sqrt, "sqrt", 1),
    (ExprFunc::Log, "log", 1),
    (ExprFunc::Log10, "log10", 1),
    (ExprFunc::Exp, "exp", 1),
    (ExprFunc::Floor, "floor", 1),
    (ExprFunc::Round, "round", 1),
    (ExprFunc::Ceil, "ceil", 1),
    (ExprFunc::Asin, "asin", 1),
    (ExprFunc::Acos, "acos", 1),
    (ExprFunc::Atan, "atan", 1),
    (ExprFunc::Atan2, "atan2", 2),
    (ExprFunc::Sinh, "sinh", 1),
    (ExprFunc::Cosh, "cosh", 1),
    (ExprFunc::Tanh, "tanh", 1),
    (ExprFunc::Logb, "logb", 1),
    (ExprFunc::Exp2, "exp2", 1),
    (ExprFunc::Log2, "log2", 1),
    (ExprFunc::Hypot, "hypot", 2),
    (ExprFunc::Cbrt, "cbrt", 1),
    (ExprFunc::Trunc, "trunc", 1),
    (ExprFunc::Min, "min", 2),
    (ExprFunc::Max, "max", 2),
    (ExprFunc::Pi, "pi", 0),
];

fn func_arity(f: ExprFunc) -> u32 {
    for &(ef, _, a) in FUNC_TABLE_ARRAY {
        if ef == f { return a; }
    }
    0
}

fn func_name(f: ExprFunc) -> &'static str {
    for &(ef, n, _) in FUNC_TABLE_ARRAY {
        if ef == f { return n; }
    }
    "unknown"
}

fn eval_func(f: ExprFunc, a: f32, b: f32) -> f32 {
    match f {
        ExprFunc::Pow => a.powf(b),
        ExprFunc::Sin => a.sin(),
        ExprFunc::Cos => a.cos(),
        ExprFunc::Tan => a.tan(),
        ExprFunc::Abs => a.abs(),
        ExprFunc::Sqrt => a.sqrt(),
        ExprFunc::Log => a.ln(),
        ExprFunc::Log10 => a.log10(),
        ExprFunc::Exp => a.exp(),
        ExprFunc::Floor => a.floor(),
        ExprFunc::Round => a.round(),
        ExprFunc::Ceil => a.ceil(),
        ExprFunc::Asin => a.asin(),
        ExprFunc::Acos => a.acos(),
        ExprFunc::Atan => a.atan(),
        ExprFunc::Atan2 => a.atan2(b),
        ExprFunc::Sinh => a.sinh(),
        ExprFunc::Cosh => a.cosh(),
        ExprFunc::Tanh => a.tanh(),
        ExprFunc::Logb => { let bits = a.to_bits(); let exp = ((bits >> 23) & 0xff) as i32; (exp - 127) as f32 },
        ExprFunc::Exp2 => (2.0f32).powf(a),
        ExprFunc::Log2 => a.log2(),
        ExprFunc::Hypot => a.hypot(b),
        ExprFunc::Cbrt => a.cbrt(),
        ExprFunc::Trunc => a.trunc(),
        ExprFunc::Min => minf(a, b),
        ExprFunc::Max => maxf(a, b),
        ExprFunc::Pi => PI,
        _ => 0.0,
    }
}

fn func_lookup_by_name(s: &str) -> ExprFunc {
    for &(ef, name, _) in FUNC_TABLE_ARRAY {
        if name == s { return ef; }
    }
    ExprFunc::Unknown
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenType {
    Float,
    Int,
    Op,
    OpenParen,
    CloseParen,
    Var,
    OpenSquare,
    CloseSquare,
    OpenCurly,
    CloseCurly,
    Func,
    Comma,
    End,
    ToFloat,
    ToInt32,
}
#[derive(Debug, Clone, Copy)]
struct Token {
    token_type: TokenType,
    value: Option<f32>,
    int_value: Option<i32>,
    var: Option<char>,
    op: Option<char>,
}

impl Token {
    fn new(tt: TokenType) -> Token {
        Token { token_type: tt, value: None, int_value: None, var: None, op: None }
    }
    fn float(f: f32) -> Token {
        Token { token_type: TokenType::Float, value: Some(f), int_value: None, var: None, op: None }
    }
    fn int(i: i32) -> Token {
        Token { token_type: TokenType::Int, value: None, int_value: Some(i), var: None, op: None }
    }
    fn op(c: char) -> Token {
        Token { token_type: TokenType::Op, value: None, int_value: None, var: None, op: Some(c) }
    }
    fn var(c: char) -> Token {
        Token { token_type: TokenType::Var, value: None, int_value: None, var: Some(c), op: None }
    }
    fn func(f: ExprFunc) -> Token {
        Token { token_type: TokenType::Func, value: None, int_value: Some(f as i32), var: None, op: None }
    }
    fn get_func(&self) -> ExprFunc {
        let i = self.int_value.unwrap_or(-1);
        unsafe { std::mem::transmute(i) }
    }
}

fn function_lookup(s: &str) -> Option<&'static FunctionEntry> {
    FUNCTION_TABLE.get(s)
}

fn expr_lex(s: Vec<&str>) -> Vec<Token> {
    // Not used by the actual parser; the parser uses expr_lex_one
    Vec::new()
}

// Lexer that works on a byte slice with a position, matching C behavior
fn expr_lex_one(bytes: &[u8], pos: &mut usize) -> Result<Token, String> {
    let len = bytes.len();
    if *pos >= len {
        return Ok(Token::new(TokenType::End));
    }

    let mut c = bytes[*pos] as char;
    let mut integer_found = false;
    let mut n: i32 = 0;

    // Skip whitespace
    loop {
        match c {
            ' ' | '\t' | '\r' | '\n' => {
                *pos += 1;
                if *pos >= len { return Ok(Token::new(TokenType::End)); }
                c = bytes[*pos] as char;
            }
            _ => break,
        }
    }

    if c.is_ascii_digit() {
        let start = *pos;
        while *pos < len && (bytes[*pos] as char).is_ascii_digit() {
            *pos += 1;
        }
        let s = std::str::from_utf8(&bytes[start..*pos]).unwrap();
        n = s.parse::<i32>().unwrap_or(0);
        integer_found = true;
        if *pos >= len || bytes[*pos] as char != '.' {
            return Ok(Token::int(n));
        }
        c = bytes[*pos] as char;
    }

    match c {
        '.' => {
            let dot_pos = *pos;
            *pos += 1;
            if *pos >= len || !(bytes[*pos] as char).is_ascii_digit() {
                if integer_found {
                    return Ok(Token::float(n as f32));
                }
                return Err(format!("unexpected '.' in lexer"));
            }
            let start = dot_pos;
            while *pos < len && (bytes[*pos] as char).is_ascii_digit() {
                *pos += 1;
            }
            let s = std::str::from_utf8(&bytes[start..*pos]).unwrap();
            let frac: f32 = s.parse().unwrap_or(0.0);
            return Ok(Token::float(n as f32 + frac));
        }
        '+' | '-' | '/' | '*' | '=' => {
            *pos += 1;
            return Ok(Token::op(c));
        }
        '(' => { *pos += 1; return Ok(Token::new(TokenType::OpenParen)); }
        ')' => { *pos += 1; return Ok(Token::new(TokenType::CloseParen)); }
        'x' | 'y' => {
            // Check if it's a variable or start of a function name
            // In C, 'x' and 'y' are always variables
            *pos += 1;
            return Ok(Token::var(c));
        }
        '[' => { *pos += 1; return Ok(Token::new(TokenType::OpenSquare)); }
        ']' => { *pos += 1; return Ok(Token::new(TokenType::CloseSquare)); }
        '{' => { *pos += 1; return Ok(Token::new(TokenType::OpenCurly)); }
        '}' => { *pos += 1; return Ok(Token::new(TokenType::CloseCurly)); }
        ',' => { *pos += 1; return Ok(Token::new(TokenType::Comma)); }
        _ => {
            if !c.is_ascii_alphabetic() {
                return Err(format!("unknown character '{}' in lexer", c));
            }
            let start = *pos;
            while *pos < len && (bytes[*pos] as char).is_ascii_alphanumeric() {
                *pos += 1;
            }
            let name = std::str::from_utf8(&bytes[start..*pos]).unwrap();
            let f = func_lookup_by_name(name);
            return Ok(Token::func(f));
        }
    }
}

pub struct ExprNode{
    pub tok: Token,
    pub is_float: i32,
    pub history_index: i32,
    pub vector_index: i32,
    pub next: Option<Arc<ExprNode>>,
}
pub struct MapperExpr{
    pub node: ExprNode,
    pub vector_size: i32,
    pub history_size: i32,
    pub history_pos: i32,
    pub input_history: Vec<MapperSignalValue>,
    pub output_history: Vec<MapperSignalValue>,
}
pub enum state_t{
    YEQUAL_Y,
    YEQUAL_EQ,
    EXPR,
    EXPR_RIGHT,
    TERM,
    TERM_RIGHT,
    VALUE,
    NEGATE,
    VAR_RIGHT,
    VAR_VECTINDEX,
    VAR_HISTINDEX,
    CLOSE_VECTINDEX,
    CLOSE_HISTINDEX,
    OPEN_PAREN,
    CLOSE_PAREN,
    COMMA,
    END,
}
enum stack_obj_t {
    State(state_t),
    Node(ExprNode),
}

impl ExprNode{
    pub fn new() -> ExprNode{
        ExprNode {
            tok: Token::new(TokenType::End),
            is_float: 0,
            history_index: 0,
            vector_index: 0,
            next: None,
        }
    }
    pub fn expr_free(&self){
        // Rust handles memory via Drop; nothing to do
    }
}

fn exprnode_from_tok(tok: &Token, is_float: i32) -> ExprNode {
    ExprNode {
        tok: *tok,
        is_float,
        history_index: 0,
        vector_index: 0,
        next: None,
    }
}

// Convert ExprNode linked list to Vec for easier manipulation
fn node_to_vec(node: ExprNode) -> Vec<ExprNode> {
    let mut result = Vec::new();
    let mut current = Some(node);
    while let Some(mut n) = current {
        let next = n.next.take();
        result.push(n);
        current = next.map(|arc| Arc::try_unwrap(arc).unwrap_or_else(|a| {
            // Clone the node chain
            let r = &*a;
            ExprNode {
                tok: r.tok,
                is_float: r.is_float,
                history_index: r.history_index,
                vector_index: r.vector_index,
                next: r.next.clone(),
            }
        }));
    }
    result
}

// Convert Vec back to linked list
fn vec_to_node(mut v: Vec<ExprNode>) -> Option<ExprNode> {
    if v.is_empty() { return None; }
    v.reverse();
    let mut head: Option<ExprNode> = None;
    for mut n in v {
        n.next = head.map(|h| Arc::new(h));
        head = Some(n);
    }
    head
}

fn get_last_is_float(nodes: &[ExprNode]) -> i32 {
    nodes.last().map(|n| n.is_float).unwrap_or(0)
}

fn printtoken(t: &Token){
    match t.token_type {
        TokenType::Float => print!("{}", t.value.unwrap_or(0.0)),
        TokenType::Int => print!("{}", t.int_value.unwrap_or(0)),
        TokenType::Op => print!("{}", t.op.unwrap_or('?')),
        TokenType::OpenParen => print!("("),
        TokenType::CloseParen => print!(")"),
        TokenType::Var => print!("VAR({})", t.var.unwrap_or('?')),
        TokenType::OpenSquare => print!("["),
        TokenType::CloseSquare => print!("]"),
        TokenType::OpenCurly => print!("{{"),
        TokenType::CloseCurly => print!("}}"),
        TokenType::Func => print!("FUNC({})", func_name(t.get_func())),
        TokenType::Comma => print!(","),
        TokenType::End => print!("END"),
        TokenType::ToFloat => print!("(float)"),
        TokenType::ToInt32 => print!("(int32)"),
    }
}

fn printexprnode(s: &str, list: &ExprNode){
    print!("{}", s);
    let mut cur: Option<&ExprNode> = Some(list);
    let mut first = true;
    while let Some(node) = cur {
        if !first { print!(" "); }
        first = false;
        if node.is_float != 0 && node.tok.token_type != TokenType::Float && node.tok.token_type != TokenType::ToFloat {
            print!(".");
        }
        printtoken(&node.tok);
        if node.tok.token_type == TokenType::Var {
            if node.history_index < 0 { print!("{{{}}}", node.history_index); }
            if node.vector_index > -1 { print!("[{}]", node.vector_index); }
        }
        cur = node.next.as_ref().map(|a| a.as_ref());
    }
}

fn printexpr(s: &str, list: &MapperExpr){
    printexprnode(s, &list.node);
}

fn printstack(stack: &stack_obj_t, stack_size: i32){
    // Simplified - just for debug
}

// Internal helper matching C's collapse_expr_to_left(plhs, rhs, constant_folding)
fn collapse_expr_to_left_impl(lhs: &mut Vec<ExprNode>, rhs: Vec<ExprNode>, constant_folding: bool) {
    if lhs.is_empty() || rhs.is_empty() { return; }

    let mut refvar = false;
    for n in lhs.iter() {
        if n.tok.token_type == TokenType::Var { refvar = true; }
    }
    for n in rhs.iter() {
        if n.tok.token_type == TokenType::Var { refvar = true; }
    }

    let lhs_last_is_float = lhs.last().map(|n| n.is_float != 0).unwrap_or(false);
    let rhs_last_is_float = rhs.last().map(|n| n.is_float != 0).unwrap_or(false);
    let is_float = lhs_last_is_float || rhs_last_is_float;

    let mut rhs = rhs;

    // Insert float coercion if sides disagree on type
    if lhs_last_is_float && !rhs_last_is_float {
        let coerce = exprnode_from_tok(&Token::new(TokenType::ToFloat), 1);
        rhs.push(coerce);
    } else if !lhs_last_is_float && rhs_last_is_float {
        let coerce = exprnode_from_tok(&Token::new(TokenType::ToFloat), 1);
        let last_idx = lhs.len() - 1;
        lhs[last_idx].is_float = 1;
        lhs.insert(last_idx, coerce);
    }

    // Insert rhs before the trailing op of lhs
    let trailing = lhs.pop().unwrap();
    lhs.extend(rhs);
    lhs.push(trailing);

    // Constant folding
    if constant_folding && !refvar {
        let node = vec_to_node(std::mem::take(lhs)).unwrap();
        let mut tmp_expr = MapperExpr {
            node,
            vector_size: 1,
            history_size: 1,
            history_pos: -1,
            input_history: vec![MapperSignalValue::I32(0)],
            output_history: vec![MapperSignalValue::I32(0)],
        };
        let dummy = MapperSignalValue::I32(0);
        let v = mapper_expr_evaluate_internal(&mut tmp_expr, None);
        let mut result_node = if is_float {
            exprnode_from_tok(&Token::float(match v { MapperSignalValue::F(f) => f, MapperSignalValue::I32(i) => i as f32 }), 1)
        } else {
            exprnode_from_tok(&Token::int(match v { MapperSignalValue::I32(i) => i, MapperSignalValue::F(f) => f as i32 }), 0)
        };
        result_node.is_float = if is_float { 1 } else { 0 };
        *lhs = vec![result_node];
    }
}

fn collapse_expr_to_left(plhs: &mut ExprNode, constant_folding: i32){
    // This signature doesn't match C usage; actual work done by collapse_expr_to_left_impl
}

// Internal evaluator that optionally takes input
fn mapper_expr_evaluate_internal(expr: &mut MapperExpr, input_vector: Option<&[MapperSignalValue]>) -> MapperSignalValue {
    let mut stack: Vec<MapperSignalValue> = Vec::with_capacity(STACK_SIZE);

    if let Some(input) = input_vector {
        expr.history_pos = (expr.history_pos + 1) % expr.history_size;
        let base = (expr.history_pos * expr.vector_size) as usize;
        for i in 0..expr.vector_size as usize {
            if i < input.len() && base + i < expr.input_history.len() {
                expr.input_history[base + i] = input[i];
            }
        }
    }

    let mut cur: Option<&ExprNode> = Some(&expr.node);
    while let Some(node) = cur {
        match node.tok.token_type {
            TokenType::Int => {
                stack.push(MapperSignalValue::I32(node.tok.int_value.unwrap_or(0)));
            }
            TokenType::Float => {
                stack.push(MapperSignalValue::F(node.tok.value.unwrap_or(0.0)));
            }
            TokenType::Var => {
                let idx = ((node.history_index + expr.history_pos + expr.history_size) % expr.history_size);
                match node.tok.var.unwrap_or('x') {
                    'x' => {
                        let i = idx * expr.vector_size + node.vector_index;
                        if (i as usize) < expr.input_history.len() {
                            stack.push(expr.input_history[i as usize]);
                        } else {
                            stack.push(MapperSignalValue::I32(0));
                        }
                    }
                    'y' => {
                        if (idx as usize) < expr.output_history.len() {
                            stack.push(expr.output_history[idx as usize]);
                        } else {
                            stack.push(MapperSignalValue::I32(0));
                        }
                    }
                    _ => { return MapperSignalValue::I32(0); }
                }
            }
            TokenType::ToFloat => {
                if let Some(top) = stack.last_mut() {
                    *top = MapperSignalValue::F(match *top {
                        MapperSignalValue::I32(i) => i as f32,
                        MapperSignalValue::F(f) => f,
                    });
                }
            }
            TokenType::ToInt32 => {
                if let Some(top) = stack.last_mut() {
                    *top = MapperSignalValue::I32(match *top {
                        MapperSignalValue::F(f) => f as i32,
                        MapperSignalValue::I32(i) => i,
                    });
                }
            }
            TokenType::Op => {
                let right = stack.pop().unwrap_or(MapperSignalValue::I32(0));
                let left = stack.pop().unwrap_or(MapperSignalValue::I32(0));
                let op = node.tok.op.unwrap_or('+');
                if node.is_float != 0 {
                    let l = match left { MapperSignalValue::F(f) => f, MapperSignalValue::I32(i) => i as f32 };
                    let r = match right { MapperSignalValue::F(f) => f, MapperSignalValue::I32(i) => i as f32 };
                    let result = match op {
                        '+' => l + r,
                        '-' => l - r,
                        '*' => l * r,
                        '/' => l / r,
                        _ => 0.0,
                    };
                    stack.push(MapperSignalValue::F(result));
                } else {
                    let l = match left { MapperSignalValue::I32(i) => i, MapperSignalValue::F(f) => f as i32 };
                    let r = match right { MapperSignalValue::I32(i) => i, MapperSignalValue::F(f) => f as i32 };
                    let result = match op {
                        '+' => l.wrapping_add(r),
                        '-' => l.wrapping_sub(r),
                        '*' => l.wrapping_mul(r),
                        '/' => if r != 0 { l / r } else { 0 },
                        _ => 0,
                    };
                    stack.push(MapperSignalValue::I32(result));
                }
            }
            TokenType::Func => {
                let f = node.tok.get_func();
                let arity = func_arity(f);
                match arity {
                    0 => {
                        stack.push(MapperSignalValue::F(eval_func(f, 0.0, 0.0)));
                    }
                    1 => {
                        let a = stack.pop().unwrap_or(MapperSignalValue::F(0.0));
                        let af = match a { MapperSignalValue::F(f) => f, MapperSignalValue::I32(i) => i as f32 };
                        stack.push(MapperSignalValue::F(eval_func(f, af, 0.0)));
                    }
                    2 => {
                        let b = stack.pop().unwrap_or(MapperSignalValue::F(0.0));
                        let a = stack.pop().unwrap_or(MapperSignalValue::F(0.0));
                        let af = match a { MapperSignalValue::F(f) => f, MapperSignalValue::I32(i) => i as f32 };
                        let bf = match b { MapperSignalValue::F(f) => f, MapperSignalValue::I32(i) => i as f32 };
                        stack.push(MapperSignalValue::F(eval_func(f, af, bf)));
                    }
                    _ => { return MapperSignalValue::I32(0); }
                }
            }
            _ => { return MapperSignalValue::I32(0); }
        }
        cur = node.next.as_ref().map(|a| a.as_ref());
    }

    let result = stack.first().copied().unwrap_or(MapperSignalValue::I32(0));
    if input_vector.is_some() {
        let hp = expr.history_pos as usize;
        if hp < expr.output_history.len() {
            expr.output_history[hp] = result;
        }
    }
    result
}

pub fn mapper_expr_new_from_string(s: &str,
                                input_is_float: i32,
                                output_is_float: i32,
                                vector_size: i32) -> MapperExpr {
    let bytes = s.as_bytes();
    let mut pos: usize = 0;

    // Stack uses Vec<ExprNode> for node entries (flattened lists)
    enum StackEntry {
        St(state_t),
        Nd(Vec<ExprNode>),
    }

    let mut stack: Vec<StackEntry> = Vec::with_capacity(STACK_SIZE);
    let mut var_allowed = true;
    let mut oldest_samps: f32 = 0.0;

    stack.push(StackEntry::St(state_t::EXPR));
    stack.push(StackEntry::St(state_t::YEQUAL_EQ));
    stack.push(StackEntry::St(state_t::YEQUAL_Y));

    let mut tok = Token::new(TokenType::End);
    let mut next_token = true;
    let mut error = false;

    macro_rules! fail {
        ($msg:expr) => {{
            println!("{}", $msg);
            error = true;
        }};
    }

    'outer: while !stack.is_empty() && !error {
        if next_token {
            match expr_lex_one(bytes, &mut pos) {
                Ok(t) => tok = t,
                Err(e) => { fail!("Error in lexical analysis."); break; }
            }
            next_token = false;
        }

        // Check if top is a Node
        let top_is_node = matches!(stack.last(), Some(StackEntry::Nd(_)));

        if top_is_node {
            if stack.len() == 1 {
                // Success - single node on stack
                break;
            }

            let len = stack.len();
            // Check if second-from-top is a State
            let second_is_state = matches!(stack.get(len - 2), Some(StackEntry::St(_)));

            if second_is_state && len >= 3 {
                let third_is_node = matches!(stack.get(len - 3), Some(StackEntry::Nd(_)));
                if third_is_node {
                    // Check what state is between them
                    let state_match = match &stack[len - 2] {
                        StackEntry::St(state_t::EXPR_RIGHT) |
                        StackEntry::St(state_t::TERM_RIGHT) |
                        StackEntry::St(state_t::CLOSE_PAREN) => 1,
                        StackEntry::St(state_t::CLOSE_HISTINDEX) => 2,
                        StackEntry::St(state_t::CLOSE_VECTINDEX) => 3,
                        _ => 0,
                    };

                    if state_match == 1 {
                        // collapse rhs into lhs, keep state
                        let rhs = match stack.pop().unwrap() { StackEntry::Nd(v) => v, _ => Vec::new() };
                        // State stays - don't remove it
                        // Find the lhs node (it's at len-3, now at len-2 after pop)
                        let lhs_idx = len - 3;
                        if let StackEntry::Nd(lhs) = &mut stack[lhs_idx] {
                            collapse_expr_to_left_impl(lhs, rhs, true);
                        }
                        continue;
                    } else if state_match == 2 {
                        // CLOSE_HISTINDEX: set history_index on var node, keep state
                        let idx_nodes = match stack.pop().unwrap() { StackEntry::Nd(v) => v, _ => Vec::new() };
                        let hist_val = if !idx_nodes.is_empty() {
                            match idx_nodes[0].tok.token_type {
                                TokenType::Float => idx_nodes[0].tok.value.unwrap_or(0.0) as i32,
                                TokenType::Int => idx_nodes[0].tok.int_value.unwrap_or(0),
                                _ => 0,
                            }
                        } else { 0 };
                        let var_idx = len - 3;
                        if let StackEntry::Nd(var_nodes) = &mut stack[var_idx] {
                            if let Some(vn) = var_nodes.first_mut() {
                                vn.history_index = hist_val;
                                if oldest_samps > hist_val as f32 {
                                    oldest_samps = hist_val as f32;
                                }
                            }
                        }
                        continue;
                    } else if state_match == 3 {
                        // CLOSE_VECTINDEX: set vector_index on var node, keep state
                        let idx_nodes = match stack.pop().unwrap() { StackEntry::Nd(v) => v, _ => Vec::new() };
                        let vect_val = if !idx_nodes.is_empty() {
                            match idx_nodes[0].tok.token_type {
                                TokenType::Float => idx_nodes[0].tok.value.unwrap_or(0.0) as i32,
                                TokenType::Int => idx_nodes[0].tok.int_value.unwrap_or(0),
                                _ => 0,
                            }
                        } else { 0 };
                        if vect_val > 0 {
                            fail!("Vector indexing not yet implemented.");
                            break;
                        }
                        if vect_val < 0 || vect_val >= vector_size {
                            fail!("Vector index outside input size.");
                            break;
                        }
                        let var_idx = len - 3;
                        if let StackEntry::Nd(var_nodes) = &mut stack[var_idx] {
                            if let Some(vn) = var_nodes.first_mut() {
                                vn.vector_index = vect_val;
                            }
                        }
                        continue;
                    } else {
                        // swap node down past state
                        let node = stack.pop().unwrap();
                        let state = stack.pop().unwrap();
                        stack.push(node);
                        stack.push(state);
                        continue;
                    }
                } else {
                    // swap node down past state
                    let node = stack.pop().unwrap();
                    let state = stack.pop().unwrap();
                    stack.push(node);
                    stack.push(state);
                    continue;
                }
            } else if second_is_state {
                // swap node down past state
                let node = stack.pop().unwrap();
                let state = stack.pop().unwrap();
                stack.push(node);
                stack.push(state);
                continue;
            }
            continue;
        }

        // Top is a State - process it
        let state = match stack.pop().unwrap() { StackEntry::St(s) => s, _ => { break; } };

        match state {
            state_t::YEQUAL_Y => {
                if tok.token_type == TokenType::Var && tok.var == Some('y') {
                    next_token = true;
                } else {
                    fail!("Error in y= prefix.");
                }
            }
            state_t::YEQUAL_EQ => {
                if tok.token_type == TokenType::Op && tok.op == Some('=') {
                    next_token = true;
                } else {
                    fail!("Error in y= prefix.");
                }
            }
            state_t::EXPR => {
                stack.push(StackEntry::St(state_t::EXPR_RIGHT));
                stack.push(StackEntry::St(state_t::TERM));
            }
            state_t::EXPR_RIGHT => {
                if tok.token_type == TokenType::Op && (tok.op == Some('+') || tok.op == Some('-')) {
                    // APPEND_OP: append op to the node below
                    if let Some(StackEntry::Nd(nodes)) = stack.last_mut() {
                        let last_is_float = get_last_is_float(nodes);
                        let mut op_node = exprnode_from_tok(&tok, last_is_float);
                        nodes.push(op_node);
                    }
                    stack.push(StackEntry::St(state_t::EXPR));
                    next_token = true;
                }
                // else: just popped, do nothing (already popped)
            }
            state_t::TERM => {
                stack.push(StackEntry::St(state_t::TERM_RIGHT));
                stack.push(StackEntry::St(state_t::VALUE));
            }
            state_t::TERM_RIGHT => {
                if tok.token_type == TokenType::Op && (tok.op == Some('*') || tok.op == Some('/')) {
                    if let Some(StackEntry::Nd(nodes)) = stack.last_mut() {
                        let last_is_float = get_last_is_float(nodes);
                        let op_node = exprnode_from_tok(&tok, last_is_float);
                        nodes.push(op_node);
                    }
                    stack.push(StackEntry::St(state_t::TERM));
                    next_token = true;
                }
            }
            state_t::VALUE => {
                if tok.token_type == TokenType::Int {
                    stack.push(StackEntry::Nd(vec![exprnode_from_tok(&tok, 0)]));
                    next_token = true;
                } else if tok.token_type == TokenType::Float {
                    stack.push(StackEntry::Nd(vec![exprnode_from_tok(&tok, 1)]));
                    next_token = true;
                } else if tok.token_type == TokenType::Var {
                    if var_allowed {
                        stack.push(StackEntry::Nd(vec![exprnode_from_tok(&tok, input_is_float)]));
                        stack.push(StackEntry::St(state_t::VAR_RIGHT));
                        next_token = true;
                    } else {
                        fail!("Unexpected variable reference.");
                    }
                } else if tok.token_type == TokenType::OpenParen {
                    stack.push(StackEntry::St(state_t::CLOSE_PAREN));
                    stack.push(StackEntry::St(state_t::EXPR));
                    next_token = true;
                } else if tok.token_type == TokenType::Func {
                    let f = tok.get_func();
                    if f == ExprFunc::Unknown {
                        fail!("Unknown function.");
                    } else {
                        let arity = func_arity(f) as i32;
                        stack.push(StackEntry::Nd(vec![exprnode_from_tok(&tok, 1)]));
                        if arity > 0 {
                            stack.push(StackEntry::St(state_t::CLOSE_PAREN));
                            stack.push(StackEntry::St(state_t::EXPR));
                            for _ in 1..arity {
                                stack.push(StackEntry::St(state_t::COMMA));
                                stack.push(StackEntry::St(state_t::EXPR));
                            }
                            stack.push(StackEntry::St(state_t::OPEN_PAREN));
                        }
                        next_token = true;
                    }
                } else if tok.token_type == TokenType::Op && tok.op == Some('-') {
                    stack.push(StackEntry::St(state_t::NEGATE));
                    stack.push(StackEntry::St(state_t::VALUE));
                    next_token = true;
                } else {
                    fail!("Expected value.");
                }
            }
            state_t::NEGATE => {
                if let Some(StackEntry::Nd(nodes)) = stack.last_mut() {
                    let rhs = std::mem::take(nodes);
                    let zero = exprnode_from_tok(&Token::int(0), 0);
                    let minus = exprnode_from_tok(&Token::op('-'), 0);
                    *nodes = vec![zero, minus];
                    collapse_expr_to_left_impl(nodes, rhs, true);
                } else {
                    fail!("Expected to negate an expression.");
                }
            }
            state_t::VAR_RIGHT => {
                if tok.token_type == TokenType::OpenSquare {
                    stack.push(StackEntry::St(state_t::VAR_VECTINDEX));
                } else if tok.token_type == TokenType::OpenCurly {
                    stack.push(StackEntry::St(state_t::VAR_HISTINDEX));
                }
                // else: just popped, do nothing
            }
            state_t::VAR_VECTINDEX => {
                if tok.token_type == TokenType::OpenSquare {
                    var_allowed = false;
                    stack.push(StackEntry::St(state_t::CLOSE_VECTINDEX));
                    stack.push(StackEntry::St(state_t::EXPR));
                    next_token = true;
                }
            }
            state_t::VAR_HISTINDEX => {
                if tok.token_type == TokenType::OpenCurly {
                    var_allowed = false;
                    stack.push(StackEntry::St(state_t::CLOSE_HISTINDEX));
                    stack.push(StackEntry::St(state_t::EXPR));
                    next_token = true;
                }
            }
            state_t::CLOSE_VECTINDEX => {
                if tok.token_type == TokenType::CloseSquare {
                    var_allowed = true;
                    stack.push(StackEntry::St(state_t::VAR_HISTINDEX));
                    next_token = true;
                } else {
                    fail!("Expected ']'.");
                }
            }
            state_t::CLOSE_HISTINDEX => {
                if tok.token_type == TokenType::CloseCurly {
                    var_allowed = true;
                    stack.push(StackEntry::St(state_t::VAR_VECTINDEX));
                    next_token = true;
                } else {
                    fail!("Expected '}'.");
                }
            }
            state_t::CLOSE_PAREN => {
                if tok.token_type == TokenType::CloseParen {
                    next_token = true;
                } else {
                    fail!("Expected ')'.");
                }
            }
            state_t::COMMA => {
                if tok.token_type == TokenType::Comma {
                    // Find previous expression node on stack and collapse current into it
                    let top_node = match stack.pop() {
                        Some(StackEntry::Nd(v)) => v,
                        _ => { fail!("Expected expression before comma."); continue; }
                    };
                    // Find the previous node entry
                    let mut found = false;
                    for i in (0..stack.len()).rev() {
                        if matches!(stack[i], StackEntry::Nd(_)) {
                            if let StackEntry::Nd(prev) = &mut stack[i] {
                                collapse_expr_to_left_impl(prev, top_node, false);
                            }
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        fail!("No previous expression for comma.");
                    }
                    next_token = true;
                } else {
                    fail!("Expected ','.");
                }
            }
            state_t::OPEN_PAREN => {
                if tok.token_type == TokenType::OpenParen {
                    next_token = true;
                } else {
                    fail!("Expected '('.");
                }
            }
            state_t::END => {
                if tok.token_type != TokenType::End {
                    fail!("Expected END.");
                }
            }
        }
    }

    let make_dummy = || -> MapperExpr {
        MapperExpr {
            node: exprnode_from_tok(&Token::int(0), 0),
            vector_size: 1,
            history_size: 1,
            history_pos: -1,
            input_history: vec![MapperSignalValue::I32(0)],
            output_history: vec![MapperSignalValue::I32(0)],
        }
    };

    if error {
        return make_dummy();
    }

    // Extract result from stack
    let result_nodes = match stack.into_iter().find(|e| matches!(e, StackEntry::Nd(_))) {
        Some(StackEntry::Nd(v)) => v,
        _ => return make_dummy(),
    };

    if oldest_samps < -100.0 {
        return make_dummy();
    }

    let mut result_nodes = result_nodes;

    // Coerce final output if necessary
    let last_is_float = result_nodes.last().map(|n| n.is_float != 0).unwrap_or(false);
    if last_is_float && output_is_float == 0 {
        result_nodes.push(exprnode_from_tok(&Token::new(TokenType::ToInt32), 0));
    } else if !last_is_float && output_is_float != 0 {
        let mut coerce = exprnode_from_tok(&Token::new(TokenType::ToFloat), 0);
        coerce.is_float = 1;
        result_nodes.push(coerce);
    }

    // Check vector indexing constraint
    if vector_size > 1 {
        for n in &result_nodes {
            if n.tok.token_type == TokenType::Var && n.vector_index > 0 {
                return make_dummy();
            }
        }
    }

    let node = vec_to_node(result_nodes).unwrap_or_else(|| ExprNode::new());
    let history_size = (-oldest_samps).ceil() as i32 + 1;

    MapperExpr {
        node,
        vector_size,
        history_size,
        history_pos: -1,
        input_history: vec![MapperSignalValue::I32(0); (vector_size * history_size) as usize],
        output_history: vec![MapperSignalValue::I32(0); history_size as usize],
    }
}

pub fn mapper_expr_evaluate<'a>(mapper: &mut MapperExpr,
                         input: &'a MapperSignalValue) -> MapperSignalValue {
    mapper_expr_evaluate_internal(mapper, Some(std::slice::from_ref(input)))
}
