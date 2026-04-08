use std::f32::consts::PI;
use std::collections::HashMap;
use std::sync::Arc;
use lazy_static::lazy_static;

const TRACING: bool = false;
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
// Ordered function table matching C's function_table array indices
const FUNC_TABLE_ORDERED: &[(&str, u32, fn(f32, f32) -> f32)] = &[
    ("pow", 2, f32::powf),
    ("sin", 1, |x, _| x.sin()),
    ("cos", 1, |x, _| x.cos()),
    ("tan", 1, |x, _| x.tan()),
    ("abs", 1, |x, _| x.abs()),
    ("sqrt", 1, |x, _| x.sqrt()),
    ("log", 1, |x, _| x.ln()),
    ("log10", 1, |x, _| x.log10()),
    ("exp", 1, |x, _| x.exp()),
    ("floor", 1, |x, _| x.floor()),
    ("round", 1, |x, _| x.round()),
    ("ceil", 1, |x, _| x.ceil()),
    ("asin", 1, |x, _| x.asin()),
    ("acos", 1, |x, _| x.acos()),
    ("atan", 1, |x, _| x.atan()),
    ("atan2", 2, f32::atan2),
    ("sinh", 1, |x, _| x.sinh()),
    ("cosh", 1, |x, _| x.cosh()),
    ("tanh", 1, |x, _| x.tanh()),
    ("logb", 1, |x, _| logbf(x)),
    ("exp2", 1, |x, _| x.exp2()),
    ("log2", 1, |x, _| x.log2()),
    ("hypot", 2, f32::hypot),
    ("cbrt", 1, |x, _| x.cbrt()),
    ("trunc", 1, |x, _| x.trunc()),
    ("min", 2, minf),
    ("max", 2, maxf),
    ("pi", 0, |_, _| PI),
];

fn logbf(x: f32) -> f32 {
    if x == 0.0 { return f32::NEG_INFINITY; }
    (x.abs().log2()).floor()
}

fn function_lookup_index(s: &str) -> i32 {
    for (i, entry) in FUNC_TABLE_ORDERED.iter().enumerate() {
        if entry.0 == s { return i as i32; }
    }
    -1 // FUNC_UNKNOWN
}

fn function_lookup(s: &str) -> Option<&'static FunctionEntry> {
    FUNCTION_TABLE.get(s)
}
fn expr_lex(s: Vec<&str>) -> Vec<Token> {
    let input: String = s.concat();
    let chars: Vec<char> = input.chars().collect();
    let mut pos = 0;
    let mut tokens = Vec::new();

    while pos < chars.len() {
        let c = chars[pos];
        if c.is_whitespace() {
            pos += 1;
            continue;
        }
        if c.is_ascii_digit() {
            let start = pos;
            while pos < chars.len() && chars[pos].is_ascii_digit() {
                pos += 1;
            }
            let n: i32 = input[start..pos].parse().unwrap_or(0);
            if pos < chars.len() && chars[pos] == '.' {
                let dot_pos = pos;
                pos += 1;
                if pos < chars.len() && chars[pos].is_ascii_digit() {
                    while pos < chars.len() && chars[pos].is_ascii_digit() {
                        pos += 1;
                    }
                    let f: f32 = input[start..pos].parse().unwrap_or(0.0);
                    tokens.push(Token { token_type: TokenType::Float, value: Some(f), int_value: None, var: None, op: None });
                } else {
                    // "N." with no digits after dot => float
                    tokens.push(Token { token_type: TokenType::Float, value: Some(n as f32), int_value: None, var: None, op: None });
                }
            } else {
                tokens.push(Token { token_type: TokenType::Int, value: None, int_value: Some(n), var: None, op: None });
            }
            continue;
        }
        match c {
            '.' => {
                let start = pos;
                pos += 1;
                if pos < chars.len() && chars[pos].is_ascii_digit() {
                    while pos < chars.len() && chars[pos].is_ascii_digit() {
                        pos += 1;
                    }
                    let f: f32 = input[start..pos].parse().unwrap_or(0.0);
                    tokens.push(Token { token_type: TokenType::Float, value: Some(f), int_value: None, var: None, op: None });
                }
                // else: stray dot, skip (matches C behavior of break)
            }
            '+' | '-' | '/' | '*' | '=' => {
                tokens.push(Token { token_type: TokenType::Op, value: None, int_value: None, var: None, op: Some(c) });
                pos += 1;
            }
            '(' => { tokens.push(Token { token_type: TokenType::OpenParen, value: None, int_value: None, var: None, op: None }); pos += 1; }
            ')' => { tokens.push(Token { token_type: TokenType::CloseParen, value: None, int_value: None, var: None, op: None }); pos += 1; }
            'x' | 'y' => { tokens.push(Token { token_type: TokenType::Var, value: None, int_value: None, var: Some(c), op: None }); pos += 1; }
            '[' => { tokens.push(Token { token_type: TokenType::OpenSquare, value: None, int_value: None, var: None, op: None }); pos += 1; }
            ']' => { tokens.push(Token { token_type: TokenType::CloseSquare, value: None, int_value: None, var: None, op: None }); pos += 1; }
            '{' => { tokens.push(Token { token_type: TokenType::OpenCurly, value: None, int_value: None, var: None, op: None }); pos += 1; }
            '}' => { tokens.push(Token { token_type: TokenType::CloseCurly, value: None, int_value: None, var: None, op: None }); pos += 1; }
            ',' => { tokens.push(Token { token_type: TokenType::Comma, value: None, int_value: None, var: None, op: None }); pos += 1; }
            _ if c.is_ascii_alphabetic() => {
                let start = pos;
                while pos < chars.len() && (chars[pos].is_ascii_alphanumeric()) {
                    pos += 1;
                }
                let name = &input[start..pos];
                // Store function index in int_value
                let func_idx = function_lookup_index(name);
                tokens.push(Token { token_type: TokenType::Func, value: None, int_value: Some(func_idx), var: None, op: None });
            }
            _ => {
                println!("unknown character '{}' in lexer", c);
                pos += 1;
            }
        }
    }
    tokens.push(Token { token_type: TokenType::End, value: None, int_value: None, var: None, op: None });
    tokens
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
            tok: Token { token_type: TokenType::End, value: None, int_value: None, var: None, op: None },
            is_float: 0,
            history_index: 0,
            vector_index: 0,
            next: None,
        }
    }
    pub fn expr_free(&self){
        // In Rust, dropping handles cleanup via Arc reference counting
    }
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
        TokenType::Func => {
            let idx = t.int_value.unwrap_or(-1);
            if idx >= 0 && (idx as usize) < FUNC_TABLE_ORDERED.len() {
                print!("FUNC({})", FUNC_TABLE_ORDERED[idx as usize].0);
            } else {
                print!("FUNC(unknown)");
            }
        }
        TokenType::Comma => print!(","),
        TokenType::End => print!("END"),
        TokenType::ToFloat => print!("(float)"),
        TokenType::ToInt32 => print!("(int32)"),
    }
}
fn printexprnode(s: &str, list: &ExprNode){
    print!("{}", s);
    let mut node: Option<&ExprNode> = Some(list);
    while let Some(n) = node {
        if n.is_float != 0 && n.tok.token_type != TokenType::Float && n.tok.token_type != TokenType::ToFloat {
            print!(".");
        }
        printtoken(&n.tok);
        if n.tok.token_type == TokenType::Var {
            if n.history_index < 0 { print!("{{{}}}", n.history_index); }
            if n.vector_index > -1 { print!("[{}]", n.vector_index); }
        }
        node = n.next.as_deref();
        if node.is_some() { print!(" "); }
    }
}
fn printexpr(s: &str, list: &MapperExpr){
    printexprnode(s, &list.node);
}
fn printstack(stack: &stack_obj_t, _stack_size: i32){
    // Debug only
}
fn collapse_expr_to_left(plhs: &mut ExprNode, constant_folding: i32){
    // This is a stub; the actual work is done by collapse_expr_to_left_with_rhs
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

// Find the last node in a chain, returning whether any VAR was found
fn chain_has_var(node: &ExprNode) -> bool {
    let mut cur = node;
    if cur.tok.token_type == TokenType::Var { return true; }
    while let Some(ref n) = cur.next {
        if n.tok.token_type == TokenType::Var { return true; }
        cur = n;
    }
    false
}

// Get the last node's is_float
fn last_is_float(node: &ExprNode) -> i32 {
    let mut cur = node;
    while let Some(ref n) = cur.next {
        cur = n;
    }
    cur.is_float
}

// Convert an ExprNode chain into a Vec for easier manipulation, then back
fn chain_to_vec(mut node: ExprNode) -> Vec<ExprNode> {
    let mut v = Vec::new();
    loop {
        let next = node.next.take();
        v.push(node);
        match next {
            Some(arc) => {
                // Try to unwrap the Arc; if it's the only reference we can take ownership
                match Arc::try_unwrap(arc) {
                    Ok(n) => node = n,
                    Err(_) => break,
                }
            }
            None => break,
        }
    }
    v
}

fn vec_to_chain(mut v: Vec<ExprNode>) -> ExprNode {
    let mut result = v.pop().unwrap();
    while let Some(mut node) = v.pop() {
        node.next = Some(Arc::new(result));
        result = node;
    }
    result
}

fn collapse_with_rhs(lhs: &mut Vec<ExprNode>, rhs: Vec<ExprNode>, constant_folding: bool) {
    let refvar = lhs.iter().any(|n| n.tok.token_type == TokenType::Var)
        || rhs.iter().any(|n| n.tok.token_type == TokenType::Var);

    let lhs_last_float = lhs.last().map(|n| n.is_float).unwrap_or(0);
    let rhs_last_float = rhs.last().map(|n| n.is_float).unwrap_or(0);
    let is_float = lhs_last_float != 0 || rhs_last_float != 0;

    let mut rhs_vec = rhs;

    // Insert float coercion if sides disagree on type
    if lhs_last_float != 0 && rhs_last_float == 0 {
        let coerce = ExprNode {
            tok: Token { token_type: TokenType::ToFloat, value: None, int_value: None, var: None, op: None },
            is_float: 1,
            history_index: 0,
            vector_index: 0,
            next: None,
        };
        rhs_vec.push(coerce);
    } else if lhs_last_float == 0 && rhs_last_float != 0 {
        let coerce = ExprNode {
            tok: Token { token_type: TokenType::ToFloat, value: None, int_value: None, var: None, op: None },
            is_float: 1,
            history_index: 0,
            vector_index: 0,
            next: None,
        };
        // Insert coerce before the last element of lhs
        let last_idx = lhs.len() - 1;
        lhs[last_idx].is_float = 1;
        lhs.insert(last_idx, coerce);
    }

    // Insert rhs before the last element of lhs (the trailing operator)
    let last_idx = lhs.len() - 1;
    let trailing = lhs.split_off(last_idx);
    lhs.extend(rhs_vec);
    lhs.extend(trailing);

    // Constant folding
    if constant_folding && !refvar {
        let chain = vec_to_chain(lhs.drain(..).collect());
        let mut expr = MapperExpr {
            node: chain,
            vector_size: 1,
            history_size: 1,
            history_pos: -1,
            input_history: vec![MapperSignalValue::I32(0)],
            output_history: vec![MapperSignalValue::I32(0)],
        };
        let dummy = MapperSignalValue::I32(0);
        let v = eval_internal(&expr, None);

        let result_node = if is_float {
            ExprNode {
                tok: Token { token_type: TokenType::Float, value: Some(match v { MapperSignalValue::F(f) => f, MapperSignalValue::I32(i) => i as f32 }), int_value: None, var: None, op: None },
                is_float: if is_float { 1 } else { 0 },
                history_index: 0,
                vector_index: 0,
                next: None,
            }
        } else {
            ExprNode {
                tok: Token { token_type: TokenType::Int, value: None, int_value: Some(match v { MapperSignalValue::I32(i) => i, MapperSignalValue::F(f) => f as i32 }), var: None, op: None },
                is_float: 0,
                history_index: 0,
                vector_index: 0,
                next: None,
            }
        };
        lhs.push(result_node);
    }
}
pub fn mapper_expr_new_from_string(s: &str, 
                                input_is_float: i32,
                                output_is_float: i32,
                                vector_size: i32)-> MapperExpr{
    let tokens = expr_lex(vec![s]);
    let mut tok_pos: usize = 0;
    
    // Stack-based parser matching the C implementation
    let mut stack: Vec<StackEntry> = Vec::with_capacity(STACK_SIZE);
    let mut var_allowed = true;
    let mut oldest_samps: f32 = 0.0;

    stack.push(StackEntry::St(state_t::YEQUAL_Y));
    stack.push(StackEntry::St(state_t::YEQUAL_EQ));
    stack.push(StackEntry::St(state_t::EXPR));

    // Note: C stack grows upward with top index; we use Vec where last element = top
    // But to match C's indexing (stack[top-1], stack[top-2]), let's use indices
    // We'll use a Vec<StackEntry> and index from 0, with `top` pointing to the last valid index
    
    let mut stk: Vec<StackEntry> = Vec::with_capacity(STACK_SIZE);
    stk.push(StackEntry::St(state_t::EXPR));
    stk.push(StackEntry::St(state_t::YEQUAL_EQ));
    stk.push(StackEntry::St(state_t::YEQUAL_Y));
    
    let mut top: i32 = 2; // index of top element
    let mut next_token = true;
    let mut tok = tokens[0];
    let mut error_message: Option<&str> = None;
    let mut result: Option<Vec<ExprNode>> = None;

    macro_rules! fail {
        ($msg:expr) => {{
            error_message = Some($msg);
            result = None;
            break;
        }};
    }

    loop {
        if top < 0 { break; }
        
        if next_token {
            if tok_pos < tokens.len() {
                tok = tokens[tok_pos];
                tok_pos += 1;
            }
            next_token = false;
        }

        let top_u = top as usize;
        
        // Check if top is a Node
        if let StackEntry::Nd(ref _n) = stk[top_u] {
            if top == 0 {
                // SUCCESS
                if let StackEntry::Nd(n) = stk.pop().unwrap() {
                    result = Some(n);
                }
                break;
            }
            
            if top >= 1 {
                if let StackEntry::St(ref _st) = stk[(top - 1) as usize] {
                    if top >= 2 {
                        if let StackEntry::Nd(ref _n2) = stk[(top - 2) as usize] {
                            let state_match = if let StackEntry::St(ref st) = stk[(top - 1) as usize] {
                                match st {
                                    state_t::EXPR_RIGHT | state_t::TERM_RIGHT | state_t::CLOSE_PAREN => Some(1),
                                    state_t::CLOSE_HISTINDEX => Some(2),
                                    state_t::CLOSE_VECTINDEX => Some(3),
                                    _ => None,
                                }
                            } else { None };
                            
                            match state_match {
                                Some(1) => {
                                    // collapse_expr_to_left
                                    if let StackEntry::Nd(rhs) = stk.remove(top_u) {
                                        // stk[top-1] is the state, stk[top-2] is lhs node
                                        if let StackEntry::Nd(ref mut lhs) = stk[(top - 2) as usize] {
                                            collapse_with_rhs(lhs, rhs, true);
                                        }
                                    }
                                    top -= 1; // we removed top, now top points to the state
                                    // but we also need to remove the state? No - in C: POP() just decrements top
                                    // In C: collapse then POP() which decrements top by 1
                                    // After collapse, the rhs node is merged into lhs. Then POP() removes the state? No.
                                    // C code: collapse_expr_to_left(&stack[top-2].node, stack[top].node, 1); POP();
                                    // POP just does top--. So after POP, top points to the state (top-1).
                                    // Then the loop continues and the state at new top will be processed.
                                    // Actually wait, we already removed the element from stk. Let me re-think.
                                    // stk has elements [0..top]. After removing top_u, stk.len() = top.
                                    // top should now be top-1 (pointing to the state).
                                    // That's what we set: top -= 1.
                                    continue;
                                }
                                Some(2) => {
                                    // CLOSE_HISTINDEX
                                    if let StackEntry::Nd(rhs) = stk.remove(top_u) {
                                        // rhs should be a single INT or FLOAT
                                        let hist_val = if rhs.len() == 1 {
                                            match rhs[0].tok.token_type {
                                                TokenType::Float => rhs[0].tok.value.unwrap_or(0.0) as i32,
                                                TokenType::Int => rhs[0].tok.int_value.unwrap_or(0),
                                                _ => 0,
                                            }
                                        } else { 0 };
                                        
                                        if let StackEntry::Nd(ref mut lhs) = stk[(top - 2) as usize] {
                                            if let Some(first) = lhs.first_mut() {
                                                first.history_index = hist_val;
                                                if (hist_val as f32) < oldest_samps {
                                                    oldest_samps = hist_val as f32;
                                                }
                                            }
                                        }
                                    }
                                    top -= 1;
                                    continue;
                                }
                                Some(3) => {
                                    // CLOSE_VECTINDEX
                                    if let StackEntry::Nd(rhs) = stk.remove(top_u) {
                                        let vect_val = if rhs.len() == 1 {
                                            match rhs[0].tok.token_type {
                                                TokenType::Float => rhs[0].tok.value.unwrap_or(0.0) as i32,
                                                TokenType::Int => rhs[0].tok.int_value.unwrap_or(0),
                                                _ => 0,
                                            }
                                        } else { 0 };
                                        
                                        if vect_val > 0 {
                                            fail!("Vector indexing not yet implemented.");
                                        }
                                        if vect_val < 0 || vect_val >= vector_size {
                                            fail!("Vector index outside input size.");
                                        }
                                        
                                        if let StackEntry::Nd(ref mut lhs) = stk[(top - 2) as usize] {
                                            if let Some(first) = lhs.first_mut() {
                                                first.vector_index = vect_val;
                                            }
                                        }
                                    }
                                    top -= 1;
                                    continue;
                                }
                                _ => {
                                    // swap expression down the stack
                                    stk.swap((top - 1) as usize, top_u);
                                    continue;
                                }
                            }
                        } else {
                            // stack[top-2] is not a node, swap
                            stk.swap((top - 1) as usize, top_u);
                            continue;
                        }
                    } else {
                        // top >= 1 but top < 2, swap
                        stk.swap((top - 1) as usize, top_u);
                        continue;
                    }
                }
            }
            continue;
        }

        // Top is a State
        let state = if let StackEntry::St(ref st) = stk[top_u] {
            match st {
                state_t::YEQUAL_Y => 0,
                state_t::YEQUAL_EQ => 1,
                state_t::EXPR => 2,
                state_t::EXPR_RIGHT => 3,
                state_t::TERM => 4,
                state_t::TERM_RIGHT => 5,
                state_t::VALUE => 6,
                state_t::NEGATE => 7,
                state_t::VAR_RIGHT => 8,
                state_t::VAR_VECTINDEX => 9,
                state_t::VAR_HISTINDEX => 10,
                state_t::CLOSE_VECTINDEX => 11,
                state_t::CLOSE_HISTINDEX => 12,
                state_t::OPEN_PAREN => 13,
                state_t::CLOSE_PAREN => 14,
                state_t::COMMA => 15,
                state_t::END => 16,
            }
        } else { continue; };

        match state {
            0 => { // YEQUAL_Y
                if tok.token_type == TokenType::Var && tok.var == Some('y') {
                    stk.pop(); top -= 1;
                } else {
                    fail!("Error in y= prefix.");
                }
                next_token = true;
            }
            1 => { // YEQUAL_EQ
                if tok.token_type == TokenType::Op && tok.op == Some('=') {
                    stk.pop(); top -= 1;
                } else {
                    fail!("Error in y= prefix.");
                }
                next_token = true;
            }
            2 => { // EXPR
                stk.pop(); top -= 1;
                stk.push(StackEntry::St(state_t::EXPR_RIGHT)); top += 1;
                stk.push(StackEntry::St(state_t::TERM)); top += 1;
            }
            3 => { // EXPR_RIGHT
                if tok.token_type == TokenType::Op {
                    let op = tok.op.unwrap_or(' ');
                    if op == '+' || op == '-' {
                        stk.pop(); top -= 1;
                        // APPEND_OP: find the node below and append the op
                        append_op_to_stack(&mut stk, top, &tok);
                        stk.push(StackEntry::St(state_t::EXPR)); top += 1;
                        next_token = true;
                    } else {
                        stk.pop(); top -= 1;
                    }
                } else {
                    stk.pop(); top -= 1;
                }
            }
            4 => { // TERM
                stk.pop(); top -= 1;
                stk.push(StackEntry::St(state_t::TERM_RIGHT)); top += 1;
                stk.push(StackEntry::St(state_t::VALUE)); top += 1;
            }
            5 => { // TERM_RIGHT
                if tok.token_type == TokenType::Op {
                    let op = tok.op.unwrap_or(' ');
                    if op == '*' || op == '/' {
                        stk.pop(); top -= 1;
                        append_op_to_stack(&mut stk, top, &tok);
                        stk.push(StackEntry::St(state_t::TERM)); top += 1;
                        next_token = true;
                    } else {
                        stk.pop(); top -= 1;
                    }
                } else {
                    stk.pop(); top -= 1;
                }
            }
            6 => { // VALUE
                if tok.token_type == TokenType::Int {
                    stk.pop(); top -= 1;
                    stk.push(StackEntry::Nd(vec![exprnode_from_tok(&tok, 0)])); top += 1;
                    next_token = true;
                } else if tok.token_type == TokenType::Float {
                    stk.pop(); top -= 1;
                    stk.push(StackEntry::Nd(vec![exprnode_from_tok(&tok, 1)])); top += 1;
                    next_token = true;
                } else if tok.token_type == TokenType::Var {
                    if var_allowed {
                        stk.pop(); top -= 1;
                        stk.push(StackEntry::Nd(vec![exprnode_from_tok(&tok, input_is_float)])); top += 1;
                        stk.push(StackEntry::St(state_t::VAR_RIGHT)); top += 1;
                        next_token = true;
                    } else {
                        fail!("Unexpected variable reference.");
                    }
                } else if tok.token_type == TokenType::OpenParen {
                    stk.pop(); top -= 1;
                    stk.push(StackEntry::St(state_t::CLOSE_PAREN)); top += 1;
                    stk.push(StackEntry::St(state_t::EXPR)); top += 1;
                    next_token = true;
                } else if tok.token_type == TokenType::Func {
                    stk.pop(); top -= 1;
                    let func_idx = tok.int_value.unwrap_or(-1);
                    if func_idx < 0 {
                        fail!("Unknown function.");
                    }
                    stk.push(StackEntry::Nd(vec![exprnode_from_tok(&tok, 1)])); top += 1;
                    let arity = FUNC_TABLE_ORDERED[func_idx as usize].1;
                    if arity > 0 {
                        stk.push(StackEntry::St(state_t::CLOSE_PAREN)); top += 1;
                        stk.push(StackEntry::St(state_t::EXPR)); top += 1;
                        for _ in 1..arity {
                            stk.push(StackEntry::St(state_t::COMMA)); top += 1;
                            stk.push(StackEntry::St(state_t::EXPR)); top += 1;
                        }
                        stk.push(StackEntry::St(state_t::OPEN_PAREN)); top += 1;
                    }
                    next_token = true;
                } else if tok.token_type == TokenType::Op && tok.op == Some('-') {
                    stk.pop(); top -= 1;
                    stk.push(StackEntry::St(state_t::NEGATE)); top += 1;
                    stk.push(StackEntry::St(state_t::VALUE)); top += 1;
                    next_token = true;
                } else {
                    fail!("Expected value.");
                }
            }
            7 => { // NEGATE
                stk.pop(); top -= 1;
                if let StackEntry::Nd(ref mut node_vec) = stk[top as usize] {
                    let zero_tok = Token { token_type: TokenType::Int, value: None, int_value: Some(0), var: None, op: None };
                    let minus_tok = Token { token_type: TokenType::Op, value: None, int_value: None, var: None, op: Some('-') };
                    let mut lhs = vec![exprnode_from_tok(&zero_tok, 0), exprnode_from_tok(&minus_tok, 0)];
                    let rhs = std::mem::replace(node_vec, Vec::new());
                    collapse_with_rhs(&mut lhs, rhs, true);
                    *node_vec = lhs;
                } else {
                    fail!("Expected to negate an expression.");
                }
            }
            8 => { // VAR_RIGHT
                if tok.token_type == TokenType::OpenSquare {
                    stk.pop(); top -= 1;
                    stk.push(StackEntry::St(state_t::VAR_VECTINDEX)); top += 1;
                } else if tok.token_type == TokenType::OpenCurly {
                    stk.pop(); top -= 1;
                    stk.push(StackEntry::St(state_t::VAR_HISTINDEX)); top += 1;
                } else {
                    stk.pop(); top -= 1;
                }
            }
            9 => { // VAR_VECTINDEX
                stk.pop(); top -= 1;
                if tok.token_type == TokenType::OpenSquare {
                    var_allowed = false;
                    stk.push(StackEntry::St(state_t::CLOSE_VECTINDEX)); top += 1;
                    stk.push(StackEntry::St(state_t::EXPR)); top += 1;
                    next_token = true;
                }
            }
            10 => { // VAR_HISTINDEX
                stk.pop(); top -= 1;
                if tok.token_type == TokenType::OpenCurly {
                    var_allowed = false;
                    stk.push(StackEntry::St(state_t::CLOSE_HISTINDEX)); top += 1;
                    stk.push(StackEntry::St(state_t::EXPR)); top += 1;
                    next_token = true;
                }
            }
            11 => { // CLOSE_VECTINDEX
                if tok.token_type == TokenType::CloseSquare {
                    var_allowed = true;
                    stk.pop(); top -= 1;
                    stk.push(StackEntry::St(state_t::VAR_HISTINDEX)); top += 1;
                    next_token = true;
                } else {
                    fail!("Expected ']'.");
                }
            }
            12 => { // CLOSE_HISTINDEX
                if tok.token_type == TokenType::CloseCurly {
                    var_allowed = true;
                    stk.pop(); top -= 1;
                    stk.push(StackEntry::St(state_t::VAR_VECTINDEX)); top += 1;
                    next_token = true;
                } else {
                    fail!("Expected '}'.");
                }
            }
            13 => { // OPEN_PAREN
                if tok.token_type == TokenType::OpenParen {
                    stk.pop(); top -= 1;
                    next_token = true;
                } else {
                    fail!("Expected '('.");
                }
            }
            14 => { // CLOSE_PAREN
                if tok.token_type == TokenType::CloseParen {
                    stk.pop(); top -= 1;
                    next_token = true;
                } else {
                    fail!("Expected ')'.");
                }
            }
            15 => { // COMMA
                if tok.token_type == TokenType::Comma {
                    stk.pop(); top -= 1;
                    // find previous expression on the stack
                    let top_u = top as usize;
                    if let StackEntry::Nd(rhs) = stk.remove(top_u) {
                        top -= 1;
                        // find the node below
                        let mut found = false;
                        for i in (0..=top as usize).rev() {
                            if let StackEntry::Nd(ref mut lhs) = stk[i] {
                                collapse_with_rhs(lhs, rhs, false);
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            fail!("Expected expression for comma.");
                        }
                    }
                    next_token = true;
                } else {
                    fail!("Expected ','.");
                }
            }
            16 => { // END
                if tok.token_type == TokenType::End {
                    stk.pop(); top -= 1;
                } else {
                    fail!("Expected END.");
                }
            }
            _ => {
                fail!("Unexpected parser state.");
            }
        }
    }

    let result_vec = match result {
        Some(v) => v,
        None => {
            if let Some(msg) = error_message {
                println!("{}", msg);
            }
            // Return a default/empty expression - panic since C returns NULL
            // but Rust signature requires MapperExpr
            panic!("Failed to parse expression");
        }
    };

    if oldest_samps < -100.0 {
        panic!("Expression contains history reference too old");
    }

    // Coerce final output if necessary
    let mut result_vec = result_vec;
    let last_float = result_vec.last().map(|n| n.is_float).unwrap_or(0);
    if last_float != 0 && output_is_float == 0 {
        result_vec.push(ExprNode {
            tok: Token { token_type: TokenType::ToInt32, value: None, int_value: None, var: None, op: None },
            is_float: 0,
            history_index: 0,
            vector_index: 0,
            next: None,
        });
    } else if last_float == 0 && output_is_float != 0 {
        result_vec.push(ExprNode {
            tok: Token { token_type: TokenType::ToFloat, value: None, int_value: None, var: None, op: None },
            is_float: 1,
            history_index: 0,
            vector_index: 0,
            next: None,
        });
    }

    // Check vector indexing
    if vector_size > 1 {
        for n in &result_vec {
            if n.tok.token_type == TokenType::Var && n.vector_index > 0 {
                panic!("vector indexing not yet implemented");
            }
        }
    }

    let history_size = (-oldest_samps).ceil() as i32 + 1;
    let node = vec_to_chain(result_vec);

    MapperExpr {
        node,
        vector_size,
        history_size,
        history_pos: -1,
        input_history: vec![MapperSignalValue::I32(0); (vector_size * history_size) as usize],
        output_history: vec![MapperSignalValue::I32(0); history_size as usize],
    }
}

enum StackEntry {
    St(state_t),
    Nd(Vec<ExprNode>),
}

fn append_op_to_stack(stk: &mut Vec<StackEntry>, top: i32, tok: &Token) {
    // Find the topmost node on the stack and append the op token
    for i in (0..=top as usize).rev() {
        if let StackEntry::Nd(ref mut nodes) = stk[i] {
            let is_float = nodes.last().map(|n| n.is_float).unwrap_or(0);
            let mut op_node = exprnode_from_tok(tok, is_float);
            nodes.push(op_node);
            break;
        }
    }
}

fn eval_internal(expr: &MapperExpr, input: Option<&MapperSignalValue>) -> MapperSignalValue {
    let mut stack: Vec<MapperSignalValue> = Vec::with_capacity(STACK_SIZE);
    let mut node: Option<&ExprNode> = Some(&expr.node);

    while let Some(n) = node {
        match n.tok.token_type {
            TokenType::Int => {
                stack.push(MapperSignalValue::I32(n.tok.int_value.unwrap_or(0)));
            }
            TokenType::Float => {
                stack.push(MapperSignalValue::F(n.tok.value.unwrap_or(0.0)));
            }
            TokenType::Var => {
                let idx = ((n.history_index + expr.history_pos + expr.history_size)
                    % expr.history_size) as usize;
                match n.tok.var.unwrap_or('x') {
                    'x' => {
                        let full_idx = idx * (expr.vector_size as usize) + n.vector_index as usize;
                        if full_idx < expr.input_history.len() {
                            stack.push(expr.input_history[full_idx]);
                        } else {
                            stack.push(MapperSignalValue::I32(0));
                        }
                    }
                    'y' => {
                        if idx < expr.output_history.len() {
                            stack.push(expr.output_history[idx]);
                        } else {
                            stack.push(MapperSignalValue::I32(0));
                        }
                    }
                    _ => {
                        stack.push(MapperSignalValue::I32(0));
                    }
                }
            }
            TokenType::ToFloat => {
                if let Some(top) = stack.last_mut() {
                    let val = match top {
                        MapperSignalValue::I32(i) => *i as f32,
                        MapperSignalValue::F(f) => *f,
                    };
                    *top = MapperSignalValue::F(val);
                }
            }
            TokenType::ToInt32 => {
                if let Some(top) = stack.last_mut() {
                    let val = match top {
                        MapperSignalValue::F(f) => *f as i32,
                        MapperSignalValue::I32(i) => *i,
                    };
                    *top = MapperSignalValue::I32(val);
                }
            }
            TokenType::Op => {
                let right = stack.pop().unwrap_or(MapperSignalValue::I32(0));
                let left = stack.pop().unwrap_or(MapperSignalValue::I32(0));
                let op = n.tok.op.unwrap_or('+');
                if n.is_float != 0 {
                    let lf = match left { MapperSignalValue::F(f) => f, MapperSignalValue::I32(i) => i as f32 };
                    let rf = match right { MapperSignalValue::F(f) => f, MapperSignalValue::I32(i) => i as f32 };
                    let res = match op {
                        '+' => lf + rf,
                        '-' => lf - rf,
                        '*' => lf * rf,
                        '/' => lf / rf,
                        _ => 0.0,
                    };
                    stack.push(MapperSignalValue::F(res));
                } else {
                    let li = match left { MapperSignalValue::I32(i) => i, MapperSignalValue::F(f) => f as i32 };
                    let ri = match right { MapperSignalValue::I32(i) => i, MapperSignalValue::F(f) => f as i32 };
                    let res = match op {
                        '+' => li.wrapping_add(ri),
                        '-' => li.wrapping_sub(ri),
                        '*' => li.wrapping_mul(ri),
                        '/' => if ri != 0 { li / ri } else { 0 },
                        _ => 0,
                    };
                    stack.push(MapperSignalValue::I32(res));
                }
            }
            TokenType::Func => {
                let func_idx = n.tok.int_value.unwrap_or(-1);
                if func_idx >= 0 && (func_idx as usize) < FUNC_TABLE_ORDERED.len() {
                    let entry = &FUNC_TABLE_ORDERED[func_idx as usize];
                    let arity = entry.1;
                    let func = entry.2;
                    match arity {
                        0 => {
                            stack.push(MapperSignalValue::F(func(0.0, 0.0)));
                        }
                        1 => {
                            let right = stack.pop().unwrap_or(MapperSignalValue::F(0.0));
                            let rf = match right { MapperSignalValue::F(f) => f, MapperSignalValue::I32(i) => i as f32 };
                            stack.push(MapperSignalValue::F(func(rf, 0.0)));
                        }
                        2 => {
                            let right = stack.pop().unwrap_or(MapperSignalValue::F(0.0));
                            let left = stack.pop().unwrap_or(MapperSignalValue::F(0.0));
                            let rf = match right { MapperSignalValue::F(f) => f, MapperSignalValue::I32(i) => i as f32 };
                            let lf = match left { MapperSignalValue::F(f) => f, MapperSignalValue::I32(i) => i as f32 };
                            stack.push(MapperSignalValue::F(func(lf, rf)));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        node = n.next.as_deref();
    }

    stack.first().copied().unwrap_or(MapperSignalValue::I32(0))
}

pub fn mapper_expr_evaluate<'a>(mapper: &mut MapperExpr, 
                         input: &'a MapperSignalValue) -> MapperSignalValue{
    // Update history
    mapper.history_pos = (mapper.history_pos + 1) % mapper.history_size;
    let idx = (mapper.history_pos * mapper.vector_size) as usize;
    if idx < mapper.input_history.len() {
        mapper.input_history[idx] = *input;
    }

    let result = eval_internal(mapper, Some(input));

    let hist_idx = mapper.history_pos as usize;
    if hist_idx < mapper.output_history.len() {
        mapper.output_history[hist_idx] = result;
    }

    result
}