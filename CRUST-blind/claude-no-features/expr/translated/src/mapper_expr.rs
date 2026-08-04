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
lazy_static! {
    static ref FUNCTION_NAMES: Vec<&'static str> = vec![
        "pow", "sin", "cos", "tan", "abs", "sqrt", "log", "log10",
        "exp", "floor", "round", "ceil", "min", "max", "pi"
    ];
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
    fn new_end() -> Token {
        Token { token_type: TokenType::End, value: None, int_value: None, var: None, op: None }
    }
}
fn function_lookup(s: &str) -> Option<&'static FunctionEntry> {
    FUNCTION_TABLE.get(s)
}
fn function_index(s: &str) -> i32 {
    for (i, n) in FUNCTION_NAMES.iter().enumerate() {
        if *n == s {
            return i as i32;
        }
    }
    -1
}
fn expr_lex(s: Vec<&str>) -> Vec<Token>{
    let combined: String = s.into_iter().collect();
    let chars: Vec<char> = combined.chars().collect();
    let mut tokens: Vec<Token> = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        // skip whitespace
        while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t' || chars[i] == '\r' || chars[i] == '\n') {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }
        let c = chars[i];
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let int_str: String = chars[start..i].iter().collect();
            let n: i32 = int_str.parse().unwrap_or(0);
            if i < chars.len() && chars[i] == '.' {
                let dot_pos = i;
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                if i == dot_pos + 1 {
                    // No digits after the dot, "n." -> float n
                    tokens.push(Token {
                        token_type: TokenType::Float,
                        value: Some(n as f32),
                        int_value: None,
                        var: None,
                        op: None,
                    });
                } else {
                    let float_str: String = chars[start..i].iter().collect();
                    let f: f32 = float_str.parse().unwrap_or(0.0);
                    tokens.push(Token {
                        token_type: TokenType::Float,
                        value: Some(f),
                        int_value: None,
                        var: None,
                        op: None,
                    });
                }
            } else {
                tokens.push(Token {
                    token_type: TokenType::Int,
                    value: None,
                    int_value: Some(n),
                    var: None,
                    op: None,
                });
            }
            continue;
        }
        if c == '.' {
            let start = i;
            i += 1;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            if i > start + 1 {
                let float_str: String = chars[start..i].iter().collect();
                let f: f32 = float_str.parse().unwrap_or(0.0);
                tokens.push(Token {
                    token_type: TokenType::Float,
                    value: Some(f),
                    int_value: None,
                    var: None,
                    op: None,
                });
                continue;
            } else {
                // Lone '.' - error, return what we have
                return tokens;
            }
        }
        match c {
            '+' | '-' | '/' | '*' | '=' => {
                tokens.push(Token { token_type: TokenType::Op, value: None, int_value: None, var: None, op: Some(c) });
                i += 1;
            }
            '(' => {
                tokens.push(Token { token_type: TokenType::OpenParen, value: None, int_value: None, var: None, op: None });
                i += 1;
            }
            ')' => {
                tokens.push(Token { token_type: TokenType::CloseParen, value: None, int_value: None, var: None, op: None });
                i += 1;
            }
            'x' | 'y' => {
                tokens.push(Token { token_type: TokenType::Var, value: None, int_value: None, var: Some(c), op: None });
                i += 1;
            }
            '[' => {
                tokens.push(Token { token_type: TokenType::OpenSquare, value: None, int_value: None, var: None, op: None });
                i += 1;
            }
            ']' => {
                tokens.push(Token { token_type: TokenType::CloseSquare, value: None, int_value: None, var: None, op: None });
                i += 1;
            }
            '{' => {
                tokens.push(Token { token_type: TokenType::OpenCurly, value: None, int_value: None, var: None, op: None });
                i += 1;
            }
            '}' => {
                tokens.push(Token { token_type: TokenType::CloseCurly, value: None, int_value: None, var: None, op: None });
                i += 1;
            }
            ',' => {
                tokens.push(Token { token_type: TokenType::Comma, value: None, int_value: None, var: None, op: None });
                i += 1;
            }
            _ => {
                if c.is_ascii_alphabetic() {
                    let start = i;
                    while i < chars.len() && (chars[i].is_ascii_alphabetic() || chars[i].is_ascii_digit()) {
                        i += 1;
                    }
                    let name: String = chars[start..i].iter().collect();
                    let idx = function_index(&name);
                    tokens.push(Token {
                        token_type: TokenType::Func,
                        value: None,
                        int_value: Some(idx),
                        var: None,
                        op: None,
                    });
                } else {
                    println!("unknown character '{}' in lexer", c);
                    return tokens;
                }
            }
        }
    }
    tokens.push(Token::new_end());
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
            tok: Token {
                token_type: TokenType::Int,
                value: None,
                int_value: Some(0),
                var: None,
                op: None,
            },
            is_float: 0,
            history_index: 0,
            vector_index: 0,
            next: None,
        }
    }
    pub fn expr_free(&self){
        // In Rust, memory is managed automatically via Arc/Drop.
        // No-op.
    }
}
fn printtoken(t: &Token){
    match t.token_type {
        TokenType::Float => print!("{}", t.value.unwrap_or(0.0)),
        TokenType::Int => print!("{}", t.int_value.unwrap_or(0)),
        TokenType::Op => print!("{}", t.op.unwrap_or(' ')),
        TokenType::OpenParen => print!("("),
        TokenType::CloseParen => print!(")"),
        TokenType::Var => print!("VAR({})", t.var.unwrap_or(' ')),
        TokenType::OpenSquare => print!("["),
        TokenType::CloseSquare => print!("]"),
        TokenType::OpenCurly => print!("{{"),
        TokenType::CloseCurly => print!("}}"),
        TokenType::Func => {
            let idx = t.int_value.unwrap_or(-1);
            if idx >= 0 && (idx as usize) < FUNCTION_NAMES.len() {
                print!("FUNC({})", FUNCTION_NAMES[idx as usize]);
            } else {
                print!("FUNC(?)");
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
    let mut cur: Option<&ExprNode> = Some(list);
    while let Some(n) = cur {
        if n.is_float != 0
            && n.tok.token_type != TokenType::Float
            && n.tok.token_type != TokenType::ToFloat {
            print!(".");
        }
        printtoken(&n.tok);
        if n.tok.token_type == TokenType::Var {
            if n.history_index < 0 {
                print!("{{{}}}", n.history_index);
            }
            if n.vector_index > -1 {
                print!("[{}]", n.vector_index);
            }
        }
        cur = n.next.as_ref().map(|a| a.as_ref());
        if cur.is_some() {
            print!(" ");
        }
    }
}
fn printexpr(s: &str, list: &MapperExpr){
    printexprnode(s, &list.node);
}
fn printstack(stack: &stack_obj_t, _stack_size: i32){
    match stack {
        stack_obj_t::State(_) => print!("STATE "),
        stack_obj_t::Node(n) => {
            print!("[");
            printexprnode("", n);
            print!("] ");
        }
    }
}
fn collapse_expr_to_left(_plhs: &mut ExprNode, _constant_folding: i32){
    // The internal collapsing is done with Vec-based representation;
    // see `collapse_internal`. This entry point is kept for API
    // compatibility but not used by the parser/evaluator.
}

// --------- Internal helpers (not part of public API) ---------

#[derive(Clone)]
struct LocalNode {
    tok: Token,
    is_float: i32,
    history_index: i32,
    vector_index: i32,
}

impl LocalNode {
    fn new(tok: Token, is_float: i32) -> Self {
        LocalNode { tok, is_float, history_index: 0, vector_index: 0 }
    }
}

enum InternalStackObj {
    State(state_t),
    List(Vec<LocalNode>),
}

fn vec_to_chain(nodes: Vec<LocalNode>) -> Option<ExprNode> {
    // Build chain from the back so each node owns Arc to next.
    let mut next_arc: Option<Arc<ExprNode>> = None;
    for n in nodes.into_iter().rev() {
        let node = ExprNode {
            tok: n.tok,
            is_float: n.is_float,
            history_index: n.history_index,
            vector_index: n.vector_index,
            next: next_arc.take(),
        };
        next_arc = Some(Arc::new(node));
    }
    // Unwrap the head from Arc (move out of Arc by reconstructing).
    if let Some(arc) = next_arc {
        Some(match Arc::try_unwrap(arc) {
            Ok(n) => n,
            Err(arc) => clone_node(arc.as_ref()),
        })
    } else {
        None
    }
}

fn clone_node(n: &ExprNode) -> ExprNode {
    ExprNode {
        tok: n.tok,
        is_float: n.is_float,
        history_index: n.history_index,
        vector_index: n.vector_index,
        next: n.next.clone(),
    }
}

fn chain_to_vec(head: &ExprNode) -> Vec<LocalNode> {
    let mut result = Vec::new();
    let mut cur: Option<&ExprNode> = Some(head);
    while let Some(n) = cur {
        result.push(LocalNode {
            tok: n.tok,
            is_float: n.is_float,
            history_index: n.history_index,
            vector_index: n.vector_index,
        });
        cur = n.next.as_ref().map(|a| a.as_ref());
    }
    result
}

fn evaluate_internal(
    nodes: &[LocalNode],
    input_history: &mut Vec<MapperSignalValue>,
    output_history: &mut Vec<MapperSignalValue>,
    history_pos: &mut i32,
    history_size: i32,
    vector_size: i32,
    input: Option<&MapperSignalValue>,
) -> MapperSignalValue {
    let mut stack: Vec<MapperSignalValue> = Vec::with_capacity(STACK_SIZE);

    if let Some(inp) = input {
        *history_pos = (*history_pos + 1).rem_euclid(history_size.max(1));
        let base = (*history_pos * vector_size) as usize;
        // Vector-size==1 in supported case; copy the single value.
        for k in 0..(vector_size as usize) {
            let idx = base + k;
            if idx < input_history.len() {
                input_history[idx] = *inp;
            }
        }
    }

    for node in nodes {
        match node.tok.token_type {
            TokenType::Int => {
                stack.push(MapperSignalValue::I32(node.tok.int_value.unwrap_or(0)));
            }
            TokenType::Float => {
                stack.push(MapperSignalValue::F(node.tok.value.unwrap_or(0.0)));
            }
            TokenType::Var => {
                let pos = *history_pos as i64;
                let size = history_size.max(1) as i64;
                let h_idx = ((node.history_index as i64 + pos + size) % size) as i32;
                match node.tok.var.unwrap_or('?') {
                    'x' => {
                        let i = (h_idx * vector_size + node.vector_index) as usize;
                        if i < input_history.len() {
                            stack.push(input_history[i]);
                        } else {
                            stack.push(MapperSignalValue::I32(0));
                        }
                    }
                    'y' => {
                        let i = h_idx as usize;
                        if i < output_history.len() {
                            stack.push(output_history[i]);
                        } else {
                            stack.push(MapperSignalValue::I32(0));
                        }
                    }
                    _ => return MapperSignalValue::I32(0),
                }
            }
            TokenType::ToFloat => {
                if let Some(v) = stack.last_mut() {
                    let f = match v {
                        MapperSignalValue::F(f) => *f,
                        MapperSignalValue::I32(i) => *i as f32,
                    };
                    *v = MapperSignalValue::F(f);
                }
            }
            TokenType::ToInt32 => {
                if let Some(v) = stack.last_mut() {
                    let i = match v {
                        MapperSignalValue::F(f) => *f as i32,
                        MapperSignalValue::I32(i) => *i,
                    };
                    *v = MapperSignalValue::I32(i);
                }
            }
            TokenType::Op => {
                let right = stack.pop().unwrap_or(MapperSignalValue::I32(0));
                let left = stack.pop().unwrap_or(MapperSignalValue::I32(0));
                let op = node.tok.op.unwrap_or('+');
                if node.is_float != 0 {
                    let lf = match left { MapperSignalValue::F(f) => f, MapperSignalValue::I32(i) => i as f32 };
                    let rf = match right { MapperSignalValue::F(f) => f, MapperSignalValue::I32(i) => i as f32 };
                    let r = match op {
                        '+' => lf + rf,
                        '-' => lf - rf,
                        '*' => lf * rf,
                        '/' => lf / rf,
                        _ => return MapperSignalValue::I32(0),
                    };
                    stack.push(MapperSignalValue::F(r));
                } else {
                    let li = match left { MapperSignalValue::I32(i) => i, MapperSignalValue::F(f) => f as i32 };
                    let ri = match right { MapperSignalValue::I32(i) => i, MapperSignalValue::F(f) => f as i32 };
                    let r = match op {
                        '+' => li.wrapping_add(ri),
                        '-' => li.wrapping_sub(ri),
                        '*' => li.wrapping_mul(ri),
                        '/' => if ri == 0 { 0 } else { li.wrapping_div(ri) },
                        _ => return MapperSignalValue::I32(0),
                    };
                    stack.push(MapperSignalValue::I32(r));
                }
            }
            TokenType::Func => {
                let func_idx = node.tok.int_value.unwrap_or(-1);
                if func_idx < 0 || (func_idx as usize) >= FUNCTION_NAMES.len() {
                    return MapperSignalValue::I32(0);
                }
                let name = FUNCTION_NAMES[func_idx as usize];
                let entry = match FUNCTION_TABLE.get(name) {
                    Some(e) => e,
                    None => return MapperSignalValue::I32(0),
                };
                let result = match entry.arity {
                    0 => (entry.func)(0.0, 0.0),
                    1 => {
                        let r = stack.pop().unwrap_or(MapperSignalValue::F(0.0));
                        let rf = match r {
                            MapperSignalValue::F(f) => f,
                            MapperSignalValue::I32(i) => i as f32,
                        };
                        (entry.func)(rf, 0.0)
                    }
                    2 => {
                        let r = stack.pop().unwrap_or(MapperSignalValue::F(0.0));
                        let l = stack.pop().unwrap_or(MapperSignalValue::F(0.0));
                        let lf = match l {
                            MapperSignalValue::F(f) => f,
                            MapperSignalValue::I32(i) => i as f32,
                        };
                        let rf = match r {
                            MapperSignalValue::F(f) => f,
                            MapperSignalValue::I32(i) => i as f32,
                        };
                        (entry.func)(lf, rf)
                    }
                    _ => return MapperSignalValue::I32(0),
                };
                stack.push(MapperSignalValue::F(result));
            }
            TokenType::End | TokenType::OpenParen | TokenType::CloseParen
            | TokenType::OpenSquare | TokenType::CloseSquare
            | TokenType::OpenCurly | TokenType::CloseCurly
            | TokenType::Comma => {
                return MapperSignalValue::I32(0);
            }
        }
    }

    let result = stack.last().copied().unwrap_or(MapperSignalValue::I32(0));

    if input.is_some() {
        let idx = *history_pos as usize;
        if idx < output_history.len() {
            output_history[idx] = result;
        }
    }

    result
}

fn collapse_internal(
    lhs: &mut Vec<LocalNode>,
    mut rhs: Vec<LocalNode>,
    constant_folding: bool,
    vector_size: i32,
) {
    if rhs.is_empty() {
        return;
    }
    if lhs.is_empty() {
        *lhs = rhs;
        return;
    }

    // Check for variable references on either side
    let mut refvar = false;
    for n in lhs.iter() {
        if n.tok.token_type == TokenType::Var {
            refvar = true;
            break;
        }
    }
    if !refvar {
        for n in rhs.iter() {
            if n.tok.token_type == TokenType::Var {
                refvar = true;
                break;
            }
        }
    }

    let lhs_last_idx = lhs.len() - 1;
    let rhs_last_idx = rhs.len() - 1;
    let lhs_last_is_float = lhs[lhs_last_idx].is_float != 0;
    let rhs_last_is_float = rhs[rhs_last_idx].is_float != 0;
    let is_float = lhs_last_is_float || rhs_last_is_float;

    if lhs_last_is_float && !rhs_last_is_float {
        // Append TOFLOAT to rhs
        let coerce = Token {
            token_type: TokenType::ToFloat,
            value: None,
            int_value: None,
            var: None,
            op: None,
        };
        rhs.push(LocalNode::new(coerce, 1));
    } else if !lhs_last_is_float && rhs_last_is_float {
        // Insert TOFLOAT before the last node of lhs and bump lhs_last.is_float
        let coerce = Token {
            token_type: TokenType::ToFloat,
            value: None,
            int_value: None,
            var: None,
            op: None,
        };
        let last_idx = lhs.len() - 1;
        lhs.insert(last_idx, LocalNode::new(coerce, 1));
        let new_last = lhs.len() - 1;
        lhs[new_last].is_float = 1;
    }

    // Insert rhs before the last node of lhs
    let last = lhs.pop().expect("lhs not empty");
    lhs.extend(rhs);
    lhs.push(last);

    let _ = rhs_last_idx; // silence unused

    if constant_folding && !refvar {
        // Evaluate the chain (with no input) to fold to a single value.
        let mut input_hist: Vec<MapperSignalValue> = vec![MapperSignalValue::I32(0); vector_size as usize];
        let mut output_hist: Vec<MapperSignalValue> = vec![MapperSignalValue::I32(0); 1];
        let mut hpos: i32 = 0;
        let v = evaluate_internal(
            lhs.as_slice(),
            &mut input_hist,
            &mut output_hist,
            &mut hpos,
            1,
            vector_size,
            None,
        );
        let new_token = if is_float {
            Token {
                token_type: TokenType::Float,
                value: Some(match v {
                    MapperSignalValue::F(f) => f,
                    MapperSignalValue::I32(i) => i as f32,
                }),
                int_value: None,
                var: None,
                op: None,
            }
        } else {
            Token {
                token_type: TokenType::Int,
                value: None,
                int_value: Some(match v {
                    MapperSignalValue::I32(i) => i,
                    MapperSignalValue::F(f) => f as i32,
                }),
                var: None,
                op: None,
            }
        };
        lhs.clear();
        lhs.push(LocalNode {
            tok: new_token,
            is_float: if is_float { 1 } else { 0 },
            history_index: 0,
            vector_index: 0,
        });
    }
}

fn append_op_to_top(stack: &mut Vec<InternalStackObj>, op_tok: Token) {
    if let Some(top) = stack.last_mut() {
        if let InternalStackObj::List(list) = top {
            if let Some(last) = list.last() {
                let is_float = last.is_float;
                let mut new_node = LocalNode::new(op_tok, 0);
                new_node.is_float = is_float;
                list.push(new_node);
            }
        }
    }
}

fn parse_internal(s: &str, input_is_float: i32, vector_size: i32) -> Option<(Vec<LocalNode>, f32)> {
    let tokens = expr_lex(vec![s]);
    if tokens.is_empty() {
        return None;
    }

    let mut stack: Vec<InternalStackObj> = Vec::with_capacity(STACK_SIZE);
    stack.push(InternalStackObj::State(state_t::EXPR));
    stack.push(InternalStackObj::State(state_t::YEQUAL_EQ));
    stack.push(InternalStackObj::State(state_t::YEQUAL_Y));

    let mut tok_idx: usize = 0;
    let mut next_token = true;
    let mut var_allowed = true;
    let mut oldest_samps: f32 = 0.0;

    let cur_token = |idx: usize, tokens: &Vec<Token>| -> Token {
        if idx < tokens.len() {
            tokens[idx]
        } else {
            Token::new_end()
        }
    };

    let mut tok = cur_token(tok_idx, &tokens);

    while !stack.is_empty() {
        if next_token {
            tok = cur_token(tok_idx, &tokens);
            next_token = false;
        }

        // If the top of the stack is a node, run reduction logic
        let top_is_node = matches!(stack.last(), Some(InternalStackObj::List(_)));
        if top_is_node {
            // Single item -> success
            if stack.len() == 1 {
                if let InternalStackObj::List(list) = stack.pop().unwrap() {
                    return Some((list, oldest_samps));
                }
            }
            let n = stack.len();
            // top-1 must be a state for the special handling
            let top_minus_1_is_state = matches!(stack[n - 2], InternalStackObj::State(_));
            if top_minus_1_is_state {
                if n >= 3 && matches!(stack[n - 3], InternalStackObj::List(_)) {
                    // Decide based on the state at n-2
                    let state_kind = match &stack[n - 2] {
                        InternalStackObj::State(s) => match s {
                            state_t::EXPR_RIGHT | state_t::TERM_RIGHT | state_t::CLOSE_PAREN => 1,
                            state_t::CLOSE_HISTINDEX => 2,
                            state_t::CLOSE_VECTINDEX => 3,
                            _ => 0,
                        },
                        _ => 0,
                    };
                    match state_kind {
                        1 => {
                            // collapse top into top-2
                            let top_list = match stack.pop().unwrap() {
                                InternalStackObj::List(l) => l,
                                _ => unreachable!(),
                            };
                            // Stack: [..., lhs_at(n-3), state_at(n-2)]
                            // After pop, len is n-1. lhs at index n-3.
                            let lhs_idx = n - 3;
                            if let InternalStackObj::List(lhs) = &mut stack[lhs_idx] {
                                collapse_internal(lhs, top_list, true, vector_size);
                            }
                        }
                        2 => {
                            // CLOSE_HISTINDEX: take top's value as history index for top-2's var
                            let top_list = match stack.pop().unwrap() {
                                InternalStackObj::List(l) => l,
                                _ => unreachable!(),
                            };
                            let lhs_idx = n - 3;
                            // top_list should be a single int/float node
                            let val_i32 = if let Some(node) = top_list.first() {
                                match node.tok.token_type {
                                    TokenType::Float => node.tok.value.unwrap_or(0.0) as i32,
                                    TokenType::Int => node.tok.int_value.unwrap_or(0),
                                    _ => 0,
                                }
                            } else {
                                0
                            };
                            if let InternalStackObj::List(lhs) = &mut stack[lhs_idx] {
                                if let Some(var_node) = lhs.last_mut() {
                                    var_node.history_index = val_i32;
                                    if (val_i32 as f32) < oldest_samps {
                                        oldest_samps = val_i32 as f32;
                                    }
                                }
                            }
                        }
                        3 => {
                            // CLOSE_VECTINDEX: take top's value as vector index for top-2's var
                            let top_list = match stack.pop().unwrap() {
                                InternalStackObj::List(l) => l,
                                _ => unreachable!(),
                            };
                            let lhs_idx = n - 3;
                            let val_i32 = if let Some(node) = top_list.first() {
                                match node.tok.token_type {
                                    TokenType::Float => node.tok.value.unwrap_or(0.0) as i32,
                                    TokenType::Int => node.tok.int_value.unwrap_or(0),
                                    _ => 0,
                                }
                            } else {
                                0
                            };
                            let mut fail = false;
                            if let InternalStackObj::List(lhs) = &mut stack[lhs_idx] {
                                if let Some(var_node) = lhs.last_mut() {
                                    var_node.vector_index = val_i32;
                                    if val_i32 > 0 {
                                        // vector indexing not yet implemented
                                        fail = true;
                                    } else if val_i32 < 0 || val_i32 >= vector_size {
                                        fail = true;
                                    }
                                }
                            }
                            if fail {
                                return None;
                            }
                        }
                        _ => {
                            // Some other state under a node -- swap
                            let last_idx = stack.len() - 1;
                            stack.swap(last_idx, last_idx - 1);
                        }
                    }
                } else {
                    // top-2 is not a node (or doesn't exist) - swap top with top-1
                    let last_idx = stack.len() - 1;
                    stack.swap(last_idx, last_idx - 1);
                }
            }
            continue;
        }

        // Otherwise, top is a state. Process it.
        let state = match stack.last() {
            Some(InternalStackObj::State(s)) => match s {
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
            },
            _ => return None,
        };

        match state {
            0 => {
                // YEQUAL_Y
                if tok.token_type == TokenType::Var && tok.var == Some('y') {
                    stack.pop();
                } else {
                    return None;
                }
                next_token = true;
                tok_idx += 1;
            }
            1 => {
                // YEQUAL_EQ
                if tok.token_type == TokenType::Op && tok.op == Some('=') {
                    stack.pop();
                } else {
                    return None;
                }
                next_token = true;
                tok_idx += 1;
            }
            2 => {
                // EXPR: pop, push EXPR_RIGHT, TERM
                stack.pop();
                stack.push(InternalStackObj::State(state_t::EXPR_RIGHT));
                stack.push(InternalStackObj::State(state_t::TERM));
            }
            3 => {
                // EXPR_RIGHT
                if tok.token_type == TokenType::Op {
                    stack.pop();
                    let op = tok.op.unwrap_or(' ');
                    if op == '+' || op == '-' {
                        append_op_to_top(&mut stack, tok);
                        stack.push(InternalStackObj::State(state_t::EXPR));
                        next_token = true;
                        tok_idx += 1;
                    }
                } else {
                    stack.pop();
                }
            }
            4 => {
                // TERM
                stack.pop();
                stack.push(InternalStackObj::State(state_t::TERM_RIGHT));
                stack.push(InternalStackObj::State(state_t::VALUE));
            }
            5 => {
                // TERM_RIGHT
                if tok.token_type == TokenType::Op {
                    stack.pop();
                    let op = tok.op.unwrap_or(' ');
                    if op == '*' || op == '/' {
                        append_op_to_top(&mut stack, tok);
                        stack.push(InternalStackObj::State(state_t::TERM));
                        next_token = true;
                        tok_idx += 1;
                    }
                } else {
                    stack.pop();
                }
            }
            6 => {
                // VALUE
                match tok.token_type {
                    TokenType::Int => {
                        stack.pop();
                        stack.push(InternalStackObj::List(vec![LocalNode::new(tok, 0)]));
                        next_token = true;
                        tok_idx += 1;
                    }
                    TokenType::Float => {
                        stack.pop();
                        stack.push(InternalStackObj::List(vec![LocalNode::new(tok, 1)]));
                        next_token = true;
                        tok_idx += 1;
                    }
                    TokenType::Var => {
                        if var_allowed {
                            stack.pop();
                            stack.push(InternalStackObj::List(vec![LocalNode::new(tok, input_is_float)]));
                            stack.push(InternalStackObj::State(state_t::VAR_RIGHT));
                            next_token = true;
                            tok_idx += 1;
                        } else {
                            return None;
                        }
                    }
                    TokenType::OpenParen => {
                        stack.pop();
                        stack.push(InternalStackObj::State(state_t::CLOSE_PAREN));
                        stack.push(InternalStackObj::State(state_t::EXPR));
                        next_token = true;
                        tok_idx += 1;
                    }
                    TokenType::Func => {
                        stack.pop();
                        let func_idx = tok.int_value.unwrap_or(-1);
                        if func_idx < 0 || (func_idx as usize) >= FUNCTION_NAMES.len() {
                            return None;
                        }
                        let name = FUNCTION_NAMES[func_idx as usize];
                        let entry = match FUNCTION_TABLE.get(name) {
                            Some(e) => e,
                            None => return None,
                        };
                        let arity = entry.arity;
                        stack.push(InternalStackObj::List(vec![LocalNode::new(tok, 1)]));
                        if arity > 0 {
                            stack.push(InternalStackObj::State(state_t::CLOSE_PAREN));
                            stack.push(InternalStackObj::State(state_t::EXPR));
                            for _ in 1..arity {
                                stack.push(InternalStackObj::State(state_t::COMMA));
                                stack.push(InternalStackObj::State(state_t::EXPR));
                            }
                            stack.push(InternalStackObj::State(state_t::OPEN_PAREN));
                        }
                        next_token = true;
                        tok_idx += 1;
                    }
                    TokenType::Op if tok.op == Some('-') => {
                        stack.pop();
                        stack.push(InternalStackObj::State(state_t::NEGATE));
                        stack.push(InternalStackObj::State(state_t::VALUE));
                        next_token = true;
                        tok_idx += 1;
                    }
                    _ => return None,
                }
            }
            7 => {
                // NEGATE: insert "0 -" before the just-parsed expression
                stack.pop();
                let top_idx_opt = if let Some(InternalStackObj::List(_)) = stack.last() {
                    Some(stack.len() - 1)
                } else {
                    None
                };
                if let Some(idx) = top_idx_opt {
                    if let InternalStackObj::List(list) = &mut stack[idx] {
                        let zero_tok = Token {
                            token_type: TokenType::Int,
                            value: None,
                            int_value: Some(0),
                            var: None,
                            op: None,
                        };
                        let minus_tok = Token {
                            token_type: TokenType::Op,
                            value: None,
                            int_value: None,
                            var: None,
                            op: Some('-'),
                        };
                        let mut new_list: Vec<LocalNode> = vec![
                            LocalNode::new(zero_tok, 0),
                            LocalNode::new(minus_tok, 0),
                        ];
                        let rhs = std::mem::take(list);
                        collapse_internal(&mut new_list, rhs, true, vector_size);
                        *list = new_list;
                    }
                } else {
                    return None;
                }
            }
            8 => {
                // VAR_RIGHT
                match tok.token_type {
                    TokenType::OpenSquare => {
                        stack.pop();
                        stack.push(InternalStackObj::State(state_t::VAR_VECTINDEX));
                    }
                    TokenType::OpenCurly => {
                        stack.pop();
                        stack.push(InternalStackObj::State(state_t::VAR_HISTINDEX));
                    }
                    _ => {
                        stack.pop();
                    }
                }
            }
            9 => {
                // VAR_VECTINDEX
                stack.pop();
                if tok.token_type == TokenType::OpenSquare {
                    var_allowed = false;
                    stack.push(InternalStackObj::State(state_t::CLOSE_VECTINDEX));
                    stack.push(InternalStackObj::State(state_t::EXPR));
                    next_token = true;
                    tok_idx += 1;
                }
            }
            10 => {
                // VAR_HISTINDEX
                stack.pop();
                if tok.token_type == TokenType::OpenCurly {
                    var_allowed = false;
                    stack.push(InternalStackObj::State(state_t::CLOSE_HISTINDEX));
                    stack.push(InternalStackObj::State(state_t::EXPR));
                    next_token = true;
                    tok_idx += 1;
                }
            }
            11 => {
                // CLOSE_VECTINDEX
                if tok.token_type == TokenType::CloseSquare {
                    var_allowed = true;
                    stack.pop();
                    stack.push(InternalStackObj::State(state_t::VAR_HISTINDEX));
                    next_token = true;
                    tok_idx += 1;
                } else {
                    return None;
                }
            }
            12 => {
                // CLOSE_HISTINDEX
                if tok.token_type == TokenType::CloseCurly {
                    var_allowed = true;
                    stack.pop();
                    stack.push(InternalStackObj::State(state_t::VAR_VECTINDEX));
                    next_token = true;
                    tok_idx += 1;
                } else {
                    return None;
                }
            }
            13 => {
                // OPEN_PAREN
                if tok.token_type == TokenType::OpenParen {
                    stack.pop();
                    next_token = true;
                    tok_idx += 1;
                } else {
                    return None;
                }
            }
            14 => {
                // CLOSE_PAREN
                if tok.token_type == TokenType::CloseParen {
                    stack.pop();
                    next_token = true;
                    tok_idx += 1;
                } else {
                    return None;
                }
            }
            15 => {
                // COMMA
                if tok.token_type == TokenType::Comma {
                    stack.pop();
                    // Search backwards from second-to-last for a list node
                    let len = stack.len();
                    let mut found_idx: Option<usize> = None;
                    if len >= 2 {
                        for j in (0..len - 1).rev() {
                            if matches!(&stack[j], InternalStackObj::List(_)) {
                                found_idx = Some(j);
                                break;
                            }
                        }
                    }
                    if let Some(j) = found_idx {
                        let top_list = match stack.pop().unwrap() {
                            InternalStackObj::List(l) => l,
                            _ => unreachable!(),
                        };
                        if let InternalStackObj::List(lhs) = &mut stack[j] {
                            collapse_internal(lhs, top_list, false, vector_size);
                        }
                    }
                    next_token = true;
                    tok_idx += 1;
                } else {
                    return None;
                }
            }
            16 => {
                // END
                if tok.token_type == TokenType::End {
                    stack.pop();
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }

    None
}

pub fn mapper_expr_new_from_string(s: &str,
                                input_is_float: i32,
                                output_is_float: i32,
                                vector_size: i32)-> MapperExpr{
    let _ = TRACING;
    if s.is_empty() {
        return MapperExpr {
            node: ExprNode::new(),
            vector_size: vector_size.max(1),
            history_size: 1,
            history_pos: -1,
            input_history: vec![MapperSignalValue::I32(0); vector_size.max(1) as usize],
            output_history: vec![MapperSignalValue::I32(0); 1],
        };
    }

    let parsed = parse_internal(s, input_is_float, vector_size);
    let (mut nodes, oldest_samps) = match parsed {
        Some(p) => p,
        None => {
            return MapperExpr {
                node: ExprNode::new(),
                vector_size: vector_size.max(1),
                history_size: 1,
                history_pos: -1,
                input_history: vec![MapperSignalValue::I32(0); vector_size.max(1) as usize],
                output_history: vec![MapperSignalValue::I32(0); 1],
            };
        }
    };

    if oldest_samps < -100.0 {
        trace!("Expression contains history reference of {}", oldest_samps);
        return MapperExpr {
            node: ExprNode::new(),
            vector_size: vector_size.max(1),
            history_size: 1,
            history_pos: -1,
            input_history: vec![MapperSignalValue::I32(0); vector_size.max(1) as usize],
            output_history: vec![MapperSignalValue::I32(0); 1],
        };
    }

    // Coerce the final output if necessary
    if let Some(last) = nodes.last() {
        let last_is_float = last.is_float != 0;
        if last_is_float && output_is_float == 0 {
            let coerce_tok = Token {
                token_type: TokenType::ToInt32,
                value: None,
                int_value: None,
                var: None,
                op: None,
            };
            nodes.push(LocalNode::new(coerce_tok, 0));
        } else if !last_is_float && output_is_float != 0 {
            let coerce_tok = Token {
                token_type: TokenType::ToFloat,
                value: None,
                int_value: None,
                var: None,
                op: None,
            };
            nodes.push(LocalNode::new(coerce_tok, 1));
        }
    }

    // Vector indexing fail-safe
    if vector_size > 1 {
        for n in nodes.iter() {
            if n.tok.token_type == TokenType::Var && n.vector_index > 0 {
                return MapperExpr {
                    node: ExprNode::new(),
                    vector_size: vector_size.max(1),
                    history_size: 1,
                    history_pos: -1,
                    input_history: vec![MapperSignalValue::I32(0); vector_size.max(1) as usize],
                    output_history: vec![MapperSignalValue::I32(0); 1],
                };
            }
        }
    }

    let history_size = ((-oldest_samps).ceil() as i32) + 1;
    let vsize = vector_size.max(1);
    let head = vec_to_chain(nodes).unwrap_or_else(ExprNode::new);

    MapperExpr {
        node: head,
        vector_size: vsize,
        history_size,
        history_pos: -1,
        input_history: vec![MapperSignalValue::I32(0); (vsize * history_size) as usize],
        output_history: vec![MapperSignalValue::I32(0); history_size as usize],
    }
}

pub fn mapper_expr_evaluate<'a>(mapper: &mut MapperExpr,
                         input: &'a MapperSignalValue) -> MapperSignalValue{
    let nodes = chain_to_vec(&mapper.node);
    let mut input_history = std::mem::take(&mut mapper.input_history);
    let mut output_history = std::mem::take(&mut mapper.output_history);
    let mut history_pos = mapper.history_pos;
    let history_size = mapper.history_size;
    let vector_size = mapper.vector_size;

    let result = evaluate_internal(
        nodes.as_slice(),
        &mut input_history,
        &mut output_history,
        &mut history_pos,
        history_size,
        vector_size,
        Some(input),
    );

    mapper.input_history = input_history;
    mapper.output_history = output_history;
    mapper.history_pos = history_pos;
    result
}
