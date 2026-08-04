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

// Helpers to interpret a MapperSignalValue as either type without losing info
fn val_to_f32(v: &MapperSignalValue) -> f32 {
    match v {
        MapperSignalValue::F(f) => *f,
        MapperSignalValue::I32(i) => *i as f32,
    }
}
fn val_to_i32(v: &MapperSignalValue) -> i32 {
    match v {
        MapperSignalValue::I32(i) => *i,
        MapperSignalValue::F(f) => *f as i32,
    }
}

const STACK_SIZE: usize = 256;
const TRACING: bool = false;

#[allow(unused_macros)]
macro_rules! trace {
    ($($arg:tt)*) => {
        if TRACING {
            println!("-- {}", format!($($arg)*));
        }
    };
}
#[allow(unused_macros)]
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

    // Extended table covering all C-side functions for completeness.
    static ref EXT_FUNCTION_TABLE: HashMap<&'static str, FunctionEntry> = {
        let mut m = HashMap::new();
        for (k, v) in FUNCTION_TABLE.iter() {
            m.insert(*k, *v);
        }
        m.insert("asin", FunctionEntry { name: "asin", arity: 1, func: |x, _| x.asin() });
        m.insert("acos", FunctionEntry { name: "acos", arity: 1, func: |x, _| x.acos() });
        m.insert("atan", FunctionEntry { name: "atan", arity: 1, func: |x, _| x.atan() });
        m.insert("atan2", FunctionEntry { name: "atan2", arity: 2, func: |y, x| y.atan2(x) });
        m.insert("sinh", FunctionEntry { name: "sinh", arity: 1, func: |x, _| x.sinh() });
        m.insert("cosh", FunctionEntry { name: "cosh", arity: 1, func: |x, _| x.cosh() });
        m.insert("tanh", FunctionEntry { name: "tanh", arity: 1, func: |x, _| x.tanh() });
        m.insert("logb", FunctionEntry { name: "logb", arity: 1, func: |x, _| {
            // logb returns floor(log2(|x|)) as a float
            if x == 0.0 { f32::NEG_INFINITY } else { x.abs().log2().floor() }
        }});
        m.insert("exp2", FunctionEntry { name: "exp2", arity: 1, func: |x, _| x.exp2() });
        m.insert("log2", FunctionEntry { name: "log2", arity: 1, func: |x, _| x.log2() });
        m.insert("hypot", FunctionEntry { name: "hypot", arity: 2, func: |x, y| x.hypot(y) });
        m.insert("cbrt", FunctionEntry { name: "cbrt", arity: 1, func: |x, _| x.cbrt() });
        m.insert("trunc", FunctionEntry { name: "trunc", arity: 1, func: |x, _| x.trunc() });
        m
    };

    // Stable ordering for storing function index in tokens.
    static ref FUNCTION_LIST: Vec<&'static str> = vec![
        "pow", "sin", "cos", "tan", "abs", "sqrt", "log", "log10", "exp",
        "floor", "round", "ceil", "asin", "acos", "atan", "atan2", "sinh",
        "cosh", "tanh", "logb", "exp2", "log2", "hypot", "cbrt", "trunc",
        "min", "max", "pi",
    ];
}

fn func_name_from_index(idx: i32) -> Option<&'static str> {
    if idx < 0 { return None; }
    FUNCTION_LIST.get(idx as usize).copied()
}

fn func_index_from_name(name: &str) -> i32 {
    FUNCTION_LIST.iter().position(|n| *n == name)
        .map(|i| i as i32).unwrap_or(-1)
}

fn func_entry_by_index(idx: i32) -> Option<&'static FunctionEntry> {
    let name = func_name_from_index(idx)?;
    EXT_FUNCTION_TABLE.get(name)
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
    fn empty() -> Token {
        Token {
            token_type: TokenType::End,
            value: None,
            int_value: None,
            var: None,
            op: None,
        }
    }
}

fn function_lookup(s: &str) -> Option<&'static FunctionEntry> {
    EXT_FUNCTION_TABLE.get(s)
}

fn expr_lex(s: Vec<&str>) -> Vec<Token> {
    let combined: String = s.into_iter().collect();
    let chars: Vec<char> = combined.chars().collect();
    let mut pos = 0usize;
    let mut tokens: Vec<Token> = Vec::new();

    while pos < chars.len() {
        let c = chars[pos];
        // skip whitespace
        if c == ' ' || c == '\t' || c == '\r' || c == '\n' {
            pos += 1;
            continue;
        }

        if c.is_ascii_digit() || c == '.' {
            let start = pos;
            let mut integer_found = false;
            while pos < chars.len() && chars[pos].is_ascii_digit() {
                pos += 1;
                integer_found = true;
            }
            if pos < chars.len() && chars[pos] == '.' {
                pos += 1; // consume '.'
                let frac_start = pos;
                while pos < chars.len() && chars[pos].is_ascii_digit() {
                    pos += 1;
                }
                if !integer_found && frac_start == pos {
                    // lone '.', invalid
                    eprintln!("unknown character '.' in lexer");
                    continue;
                }
                let mut numstr: String = chars[start..pos].iter().collect();
                if numstr.ends_with('.') { numstr.push('0'); }
                if numstr.starts_with('.') { numstr.insert(0, '0'); }
                let v: f32 = numstr.parse().unwrap_or(0.0);
                tokens.push(Token {
                    token_type: TokenType::Float,
                    value: Some(v),
                    int_value: None,
                    var: None,
                    op: None,
                });
            } else if integer_found {
                let numstr: String = chars[start..pos].iter().collect();
                let v: i32 = numstr.parse().unwrap_or(0);
                tokens.push(Token {
                    token_type: TokenType::Int,
                    value: None,
                    int_value: Some(v),
                    var: None,
                    op: None,
                });
            }
            continue;
        }

        match c {
            '+' | '-' | '*' | '/' | '=' => {
                tokens.push(Token {
                    token_type: TokenType::Op,
                    value: None, int_value: None, var: None, op: Some(c),
                });
                pos += 1;
            }
            '(' => {
                tokens.push(Token {
                    token_type: TokenType::OpenParen,
                    value: None, int_value: None, var: None, op: None,
                });
                pos += 1;
            }
            ')' => {
                tokens.push(Token {
                    token_type: TokenType::CloseParen,
                    value: None, int_value: None, var: None, op: None,
                });
                pos += 1;
            }
            '[' => {
                tokens.push(Token {
                    token_type: TokenType::OpenSquare,
                    value: None, int_value: None, var: None, op: None,
                });
                pos += 1;
            }
            ']' => {
                tokens.push(Token {
                    token_type: TokenType::CloseSquare,
                    value: None, int_value: None, var: None, op: None,
                });
                pos += 1;
            }
            '{' => {
                tokens.push(Token {
                    token_type: TokenType::OpenCurly,
                    value: None, int_value: None, var: None, op: None,
                });
                pos += 1;
            }
            '}' => {
                tokens.push(Token {
                    token_type: TokenType::CloseCurly,
                    value: None, int_value: None, var: None, op: None,
                });
                pos += 1;
            }
            ',' => {
                tokens.push(Token {
                    token_type: TokenType::Comma,
                    value: None, int_value: None, var: None, op: None,
                });
                pos += 1;
            }
            'x' | 'y' => {
                tokens.push(Token {
                    token_type: TokenType::Var,
                    value: None, int_value: None, var: Some(c), op: None,
                });
                pos += 1;
            }
            _ => {
                if c.is_ascii_alphabetic() {
                    let start = pos;
                    while pos < chars.len()
                        && (chars[pos].is_ascii_alphabetic()
                            || chars[pos].is_ascii_digit())
                    {
                        pos += 1;
                    }
                    let name: String = chars[start..pos].iter().collect();
                    let idx = func_index_from_name(&name);
                    tokens.push(Token {
                        token_type: TokenType::Func,
                        value: None,
                        int_value: Some(idx),
                        var: None,
                        op: None,
                    });
                } else {
                    eprintln!("unknown character '{}' in lexer", c);
                    pos += 1;
                }
            }
        }
    }

    tokens.push(Token {
        token_type: TokenType::End,
        value: None, int_value: None, var: None, op: None,
    });
    tokens
}

pub struct ExprNode {
    pub tok: Token,
    pub is_float: i32,
    pub history_index: i32,
    pub vector_index: i32,
    pub next: Option<Arc<ExprNode>>,
}

pub struct MapperExpr {
    pub node: ExprNode,
    pub vector_size: i32,
    pub history_size: i32,
    pub history_pos: i32,
    pub input_history: Vec<MapperSignalValue>,
    pub output_history: Vec<MapperSignalValue>,
}

pub enum state_t {
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

#[allow(dead_code)]
enum stack_obj_t {
    State(state_t),
    Node(ExprNode),
}

impl ExprNode {
    pub fn new() -> ExprNode {
        ExprNode {
            tok: Token::empty(),
            is_float: 0,
            history_index: 0,
            vector_index: 0,
            next: None,
        }
    }
    pub fn expr_free(&self) {
        // Rust handles memory automatically.
    }
}

fn printtoken(_t: &Token) {
    // Debug helper; not used in non-debug builds.
}
fn printexprnode(_s: &str, _list: &ExprNode) {
    // Debug helper; not used in non-debug builds.
}
fn printexpr(_s: &str, _list: &MapperExpr) {
    // Debug helper; not used in non-debug builds.
}
fn printstack(_stack: &stack_obj_t, _stack_size: i32) {
    // Debug helper; not used in non-debug builds.
}

fn collapse_expr_to_left(_plhs: &mut ExprNode, _constant_folding: i32) {
    // Public-shape stub; the real splice-and-fold logic is implemented by
    // collapse_internal which operates over the parser's internal Vec form.
}

// ---- Internal node representation for parsing ----

#[derive(Clone)]
struct InternalNode {
    tok: Token,
    is_float: i32,
    history_index: i32,
    vector_index: i32,
}

impl InternalNode {
    fn from_token(tok: Token, is_float: i32) -> InternalNode {
        InternalNode {
            tok,
            is_float,
            history_index: 0,
            vector_index: 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum InternalState {
    YequalY,
    YequalEq,
    Expr,
    ExprRight,
    Term,
    TermRight,
    Value,
    Negate,
    VarRight,
    VarVectIndex,
    VarHistIndex,
    CloseVectIndex,
    CloseHistIndex,
    OpenParen,
    CloseParen,
    Comma,
    End,
}

enum ParserStackObj {
    State(InternalState),
    Node(Vec<InternalNode>),
}

fn vec_to_arc_chain(nodes: &[InternalNode]) -> Option<Arc<ExprNode>> {
    if nodes.is_empty() {
        return None;
    }
    let mut head: Option<Arc<ExprNode>> = None;
    for n in nodes.iter().rev() {
        let new_node = ExprNode {
            tok: n.tok,
            is_float: n.is_float,
            history_index: n.history_index,
            vector_index: n.vector_index,
            next: head,
        };
        head = Some(Arc::new(new_node));
    }
    head
}

fn vec_to_expr_node(nodes: &[InternalNode]) -> ExprNode {
    if nodes.is_empty() {
        return ExprNode::new();
    }
    let first = &nodes[0];
    let rest = vec_to_arc_chain(&nodes[1..]);
    ExprNode {
        tok: first.tok,
        is_float: first.is_float,
        history_index: first.history_index,
        vector_index: first.vector_index,
        next: rest,
    }
}

// Splice the rhs node list into the lhs node list, immediately before the
// trailing operator that already lives at the end of lhs. Optionally fold the
// resulting subexpression to a constant if it has no variable references.
fn collapse_internal(plhs: &mut Vec<InternalNode>, rhs: Vec<InternalNode>, constant_folding: bool) {
    if plhs.is_empty() {
        *plhs = rhs;
        return;
    }
    if rhs.is_empty() {
        return;
    }

    let mut rhs = rhs;
    let mut refvar = false;

    for n in &rhs {
        if n.tok.token_type == TokenType::Var {
            refvar = true;
        }
    }
    for n in plhs.iter() {
        if n.tok.token_type == TokenType::Var {
            refvar = true;
        }
    }

    // The "type" of each side is the trailing op's marker. Mirrors the C code
    // which examines (*plhs_last)->is_float and rhs_last->is_float.
    let plhs_last_is_float = plhs.last().map(|n| n.is_float != 0).unwrap_or(false);
    let rhs_last_is_float = rhs.last().map(|n| n.is_float != 0).unwrap_or(false);
    let is_float = plhs_last_is_float || rhs_last_is_float;

    let coerce_token = Token {
        token_type: TokenType::ToFloat,
        value: None,
        int_value: None,
        var: None,
        op: None,
    };

    if plhs_last_is_float && !rhs_last_is_float {
        // append (float) coerce after rhs
        rhs.push(InternalNode {
            tok: coerce_token,
            is_float: 1,
            history_index: 0,
            vector_index: 0,
        });
    } else if !plhs_last_is_float && rhs_last_is_float {
        // insert (float) coerce before the trailing op of plhs and mark op as float
        let last_idx = plhs.len() - 1;
        plhs[last_idx].is_float = 1;
        plhs.insert(
            last_idx,
            InternalNode {
                tok: coerce_token,
                is_float: 1,
                history_index: 0,
                vector_index: 0,
            },
        );
    }

    // Splice: insert rhs before the trailing op (last node) of lhs.
    let last_idx = plhs.len() - 1;
    let trailing = plhs.remove(last_idx);
    plhs.extend(rhs);
    plhs.push(trailing);

    if constant_folding && !refvar {
        // Build a temp expression and evaluate it now.
        let node = vec_to_expr_node(plhs);
        let mut tmp = MapperExpr {
            node,
            vector_size: 1,
            history_size: 1,
            history_pos: -1,
            input_history: vec![MapperSignalValue::F(0.0)],
            output_history: vec![MapperSignalValue::F(0.0)],
        };
        let dummy = MapperSignalValue::F(0.0);
        let v = mapper_expr_evaluate_internal(&mut tmp, None, &dummy);

        plhs.clear();
        if is_float {
            let f = val_to_f32(&v);
            plhs.push(InternalNode {
                tok: Token {
                    token_type: TokenType::Float,
                    value: Some(f),
                    int_value: None,
                    var: None,
                    op: None,
                },
                is_float: 1,
                history_index: 0,
                vector_index: 0,
            });
        } else {
            let i = val_to_i32(&v);
            plhs.push(InternalNode {
                tok: Token {
                    token_type: TokenType::Int,
                    value: None,
                    int_value: Some(i),
                    var: None,
                    op: None,
                },
                is_float: 0,
                history_index: 0,
                vector_index: 0,
            });
        }
    }
}

// Append a new operator token to the trailing position of the topmost node
// list on the stack. Mirrors the APPEND_OP macro in the C parser.
fn append_op(stack: &mut Vec<ParserStackObj>, tok: Token) {
    if let Some(ParserStackObj::Node(nodes)) = stack.last_mut() {
        if let Some(last) = nodes.last() {
            let is_float = last.is_float;
            nodes.push(InternalNode {
                tok,
                is_float,
                history_index: 0,
                vector_index: 0,
            });
        }
    }
}

// ---- Parser entry point ----

pub fn mapper_expr_new_from_string(
    s: &str,
    input_is_float: i32,
    output_is_float: i32,
    vector_size: i32,
) -> MapperExpr {
    // Lex first so we can iterate over tokens by index.
    let tokens = expr_lex(vec![s]);
    let mut tok_idx: usize = 0;
    let mut next_token = true;
    let mut tok: Token = Token::empty();

    let mut stack: Vec<ParserStackObj> = Vec::new();
    let mut error_message: Option<&'static str> = None;
    let mut result: Option<Vec<InternalNode>> = None;

    let mut var_allowed = true;
    let mut oldest_samps: f32 = 0.0;

    stack.push(ParserStackObj::State(InternalState::Expr));
    stack.push(ParserStackObj::State(InternalState::YequalEq));
    stack.push(ParserStackObj::State(InternalState::YequalY));

    'outer: while !stack.is_empty() {
        if next_token {
            if tok_idx >= tokens.len() {
                error_message = Some("Error in lexical analysis.");
                break;
            }
            tok = tokens[tok_idx];
            tok_idx += 1;
            next_token = false;
        }

        // If top of stack is a Node, attempt to combine with what's below.
        let top_is_node = matches!(stack.last(), Some(ParserStackObj::Node(_)));
        if top_is_node {
            let stack_len = stack.len();
            if stack_len == 1 {
                if let Some(ParserStackObj::Node(nodes)) = stack.pop() {
                    result = Some(nodes);
                }
                break;
            }
            // Look at the entry directly below the top.
            let below = &stack[stack_len - 2];
            match below {
                ParserStackObj::State(state) => {
                    let state = *state;
                    if stack_len >= 3 && matches!(stack[stack_len - 3], ParserStackObj::Node(_)) {
                        match state {
                            InternalState::ExprRight
                            | InternalState::TermRight
                            | InternalState::CloseParen => {
                                // collapse top into [top-2]
                                let top_node = match stack.pop().unwrap() {
                                    ParserStackObj::Node(n) => n,
                                    _ => unreachable!(),
                                };
                                // pop the state momentarily
                                let st = stack.pop().unwrap();
                                if let Some(ParserStackObj::Node(lhs)) = stack.last_mut() {
                                    collapse_internal(lhs, top_node, true);
                                }
                                stack.push(st);
                            }
                            InternalState::CloseHistIndex => {
                                // pop top node, expect lonely INT/FLOAT
                                let top_node = match stack.pop().unwrap() {
                                    ParserStackObj::Node(n) => n,
                                    _ => unreachable!(),
                                };
                                // pop state
                                stack.pop();
                                if top_node.len() != 1
                                    || (top_node[0].tok.token_type != TokenType::Int
                                        && top_node[0].tok.token_type != TokenType::Float)
                                {
                                    error_message = Some(
                                        "expected lonely INT or FLOAT expression on the stack.",
                                    );
                                    break 'outer;
                                }
                                let val: i32 = if top_node[0].tok.token_type == TokenType::Float {
                                    top_node[0].tok.value.unwrap_or(0.0) as i32
                                } else {
                                    top_node[0].tok.int_value.unwrap_or(0)
                                };
                                if let Some(ParserStackObj::Node(tn)) = stack.last_mut() {
                                    if let Some(target) = tn.last_mut() {
                                        if target.tok.token_type != TokenType::Var {
                                            error_message =
                                                Some("expected VAR two-down on the stack.");
                                            break 'outer;
                                        }
                                        target.history_index = val;
                                        if (oldest_samps as f32) > val as f32 {
                                            oldest_samps = val as f32;
                                        }
                                    } else {
                                        error_message = Some("expected VAR two-down on the stack.");
                                        break 'outer;
                                    }
                                }
                            }
                            InternalState::CloseVectIndex => {
                                let top_node = match stack.pop().unwrap() {
                                    ParserStackObj::Node(n) => n,
                                    _ => unreachable!(),
                                };
                                stack.pop();
                                if top_node.len() != 1
                                    || (top_node[0].tok.token_type != TokenType::Int
                                        && top_node[0].tok.token_type != TokenType::Float)
                                {
                                    error_message = Some(
                                        "expected lonely INT or FLOAT expression on the stack.",
                                    );
                                    break 'outer;
                                }
                                let val: i32 = if top_node[0].tok.token_type == TokenType::Float {
                                    top_node[0].tok.value.unwrap_or(0.0) as i32
                                } else {
                                    top_node[0].tok.int_value.unwrap_or(0)
                                };
                                if let Some(ParserStackObj::Node(tn)) = stack.last_mut() {
                                    if let Some(target) = tn.last_mut() {
                                        if target.tok.token_type != TokenType::Var {
                                            error_message =
                                                Some("expected VAR two-down on the stack.");
                                            break 'outer;
                                        }
                                        target.vector_index = val;
                                        if val > 0 {
                                            error_message =
                                                Some("Vector indexing not yet implemented.");
                                            break 'outer;
                                        }
                                        if val < 0 || val >= vector_size {
                                            error_message =
                                                Some("Vector index outside input size.");
                                            break 'outer;
                                        }
                                    } else {
                                        error_message = Some("expected VAR two-down on the stack.");
                                        break 'outer;
                                    }
                                }
                            }
                            _ => {
                                // No special combine; swap node and state below.
                                let top = stack.pop().unwrap();
                                let mid = stack.pop().unwrap();
                                stack.push(top);
                                stack.push(mid);
                            }
                        }
                    } else {
                        // Below the state is not a node — swap node down.
                        let top = stack.pop().unwrap();
                        let mid = stack.pop().unwrap();
                        stack.push(top);
                        stack.push(mid);
                    }
                }
                ParserStackObj::Node(_) => {
                    // Two nodes adjacent — mirrors C behavior of falling through
                    // to the outer `continue`. Shouldn't occur in valid programs.
                }
            }
            continue;
        }

        // Top is a state. Dispatch on it.
        let state = match stack.last() {
            Some(ParserStackObj::State(s)) => *s,
            _ => break,
        };

        match state {
            InternalState::YequalY => {
                if tok.token_type == TokenType::Var && tok.var == Some('y') {
                    stack.pop();
                } else {
                    error_message = Some("Error in y= prefix.");
                    break;
                }
                next_token = true;
            }
            InternalState::YequalEq => {
                if tok.token_type == TokenType::Op && tok.op == Some('=') {
                    stack.pop();
                } else {
                    error_message = Some("Error in y= prefix.");
                    break;
                }
                next_token = true;
            }
            InternalState::Expr => {
                stack.pop();
                stack.push(ParserStackObj::State(InternalState::ExprRight));
                stack.push(ParserStackObj::State(InternalState::Term));
            }
            InternalState::ExprRight => {
                if tok.token_type == TokenType::Op {
                    stack.pop();
                    if tok.op == Some('+') || tok.op == Some('-') {
                        append_op(&mut stack, tok);
                        stack.push(ParserStackObj::State(InternalState::Expr));
                        next_token = true;
                    }
                } else {
                    stack.pop();
                }
            }
            InternalState::Term => {
                stack.pop();
                stack.push(ParserStackObj::State(InternalState::TermRight));
                stack.push(ParserStackObj::State(InternalState::Value));
            }
            InternalState::TermRight => {
                if tok.token_type == TokenType::Op {
                    stack.pop();
                    if tok.op == Some('*') || tok.op == Some('/') {
                        append_op(&mut stack, tok);
                        stack.push(ParserStackObj::State(InternalState::Term));
                        next_token = true;
                    }
                } else {
                    stack.pop();
                }
            }
            InternalState::Value => {
                if tok.token_type == TokenType::Int {
                    stack.pop();
                    stack.push(ParserStackObj::Node(vec![InternalNode::from_token(tok, 0)]));
                    next_token = true;
                } else if tok.token_type == TokenType::Float {
                    stack.pop();
                    stack.push(ParserStackObj::Node(vec![InternalNode::from_token(tok, 1)]));
                    next_token = true;
                } else if tok.token_type == TokenType::Var {
                    if var_allowed {
                        stack.pop();
                        stack.push(ParserStackObj::Node(vec![InternalNode::from_token(
                            tok,
                            input_is_float,
                        )]));
                        stack.push(ParserStackObj::State(InternalState::VarRight));
                        next_token = true;
                    } else {
                        error_message = Some("Unexpected variable reference.");
                        break;
                    }
                } else if tok.token_type == TokenType::OpenParen {
                    stack.pop();
                    stack.push(ParserStackObj::State(InternalState::CloseParen));
                    stack.push(ParserStackObj::State(InternalState::Expr));
                    next_token = true;
                } else if tok.token_type == TokenType::Func {
                    stack.pop();
                    let func_idx = tok.int_value.unwrap_or(-1);
                    if func_idx < 0 {
                        error_message = Some("Unknown function.");
                        break;
                    }
                    let entry = match func_entry_by_index(func_idx) {
                        Some(e) => e,
                        None => {
                            error_message = Some("Unknown function.");
                            break;
                        }
                    };
                    let arity = entry.arity;
                    stack.push(ParserStackObj::Node(vec![InternalNode::from_token(tok, 1)]));
                    if arity > 0 {
                        stack.push(ParserStackObj::State(InternalState::CloseParen));
                        stack.push(ParserStackObj::State(InternalState::Expr));
                        for _ in 1..arity {
                            stack.push(ParserStackObj::State(InternalState::Comma));
                            stack.push(ParserStackObj::State(InternalState::Expr));
                        }
                        stack.push(ParserStackObj::State(InternalState::OpenParen));
                    }
                    next_token = true;
                } else if tok.token_type == TokenType::Op && tok.op == Some('-') {
                    stack.pop();
                    stack.push(ParserStackObj::State(InternalState::Negate));
                    stack.push(ParserStackObj::State(InternalState::Value));
                    next_token = true;
                } else {
                    error_message = Some("Expected value.");
                    break;
                }
            }
            InternalState::Negate => {
                stack.pop();
                let top_is_node = matches!(stack.last(), Some(ParserStackObj::Node(_)));
                if top_is_node {
                    let top_node = match stack.pop().unwrap() {
                        ParserStackObj::Node(n) => n,
                        _ => unreachable!(),
                    };
                    // Build [0, -] and collapse top_node into it.
                    let mut prefix: Vec<InternalNode> = vec![
                        InternalNode {
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
                        },
                        InternalNode {
                            tok: Token {
                                token_type: TokenType::Op,
                                value: None,
                                int_value: None,
                                var: None,
                                op: Some('-'),
                            },
                            is_float: 0,
                            history_index: 0,
                            vector_index: 0,
                        },
                    ];
                    collapse_internal(&mut prefix, top_node, true);
                    stack.push(ParserStackObj::Node(prefix));
                } else {
                    error_message = Some("Expected to negate an expression.");
                    break;
                }
            }
            InternalState::VarRight => {
                if tok.token_type == TokenType::OpenSquare {
                    stack.pop();
                    stack.push(ParserStackObj::State(InternalState::VarVectIndex));
                } else if tok.token_type == TokenType::OpenCurly {
                    stack.pop();
                    stack.push(ParserStackObj::State(InternalState::VarHistIndex));
                } else {
                    stack.pop();
                }
            }
            InternalState::VarVectIndex => {
                stack.pop();
                if tok.token_type == TokenType::OpenSquare {
                    var_allowed = false;
                    stack.push(ParserStackObj::State(InternalState::CloseVectIndex));
                    stack.push(ParserStackObj::State(InternalState::Expr));
                    next_token = true;
                }
            }
            InternalState::VarHistIndex => {
                stack.pop();
                if tok.token_type == TokenType::OpenCurly {
                    var_allowed = false;
                    stack.push(ParserStackObj::State(InternalState::CloseHistIndex));
                    stack.push(ParserStackObj::State(InternalState::Expr));
                    next_token = true;
                }
            }
            InternalState::CloseVectIndex => {
                if tok.token_type == TokenType::CloseSquare {
                    var_allowed = true;
                    stack.pop();
                    stack.push(ParserStackObj::State(InternalState::VarHistIndex));
                    next_token = true;
                } else {
                    error_message = Some("Expected ']'.");
                    break;
                }
            }
            InternalState::CloseHistIndex => {
                if tok.token_type == TokenType::CloseCurly {
                    var_allowed = true;
                    stack.pop();
                    stack.push(ParserStackObj::State(InternalState::VarVectIndex));
                    next_token = true;
                } else {
                    error_message = Some("Expected '}'.");
                    break;
                }
            }
            InternalState::CloseParen => {
                if tok.token_type == TokenType::CloseParen {
                    stack.pop();
                    next_token = true;
                } else {
                    error_message = Some("Expected ')'.");
                    break;
                }
            }
            InternalState::Comma => {
                if tok.token_type == TokenType::Comma {
                    stack.pop();
                    // find the previous node deeper in the stack and collapse the
                    // top node into it (no constant folding on commas).
                    // First, the stack top is a state we just popped; we need to
                    // find a node above the previous-node target.
                    // Actually after pop above, the new top should be a Node (the
                    // current expression), and below could be more states/nodes.
                    let top_is_node = matches!(stack.last(), Some(ParserStackObj::Node(_)));
                    if top_is_node {
                        let top_node = match stack.pop().unwrap() {
                            ParserStackObj::Node(n) => n,
                            _ => unreachable!(),
                        };
                        // Find previous Node deeper in the stack.
                        let mut idx_opt: Option<usize> = None;
                        for i in (0..stack.len()).rev() {
                            if let ParserStackObj::Node(_) = stack[i] {
                                idx_opt = Some(i);
                                break;
                            }
                        }
                        if let Some(i) = idx_opt {
                            if let ParserStackObj::Node(ref mut lhs) = stack[i] {
                                collapse_internal(lhs, top_node, false);
                            }
                        }
                    }
                    next_token = true;
                } else {
                    error_message = Some("Expected ','.");
                    break;
                }
            }
            InternalState::OpenParen => {
                if tok.token_type == TokenType::OpenParen {
                    stack.pop();
                    next_token = true;
                } else {
                    error_message = Some("Expected '('.");
                    break;
                }
            }
            InternalState::End => {
                if tok.token_type == TokenType::End {
                    stack.pop();
                } else {
                    error_message = Some("Expected END.");
                    break;
                }
            }
        }
    }

    // If we emptied the stack but never saw a final lone Node, we may have
    // left the result on the stack already. Handle that case too.
    if result.is_none() {
        // Try to pull a final node off the stack.
        for obj in stack.into_iter().rev() {
            if let ParserStackObj::Node(n) = obj {
                result = Some(n);
                break;
            }
        }
    }

    if result.is_none() {
        if let Some(msg) = error_message {
            eprintln!("{}", msg);
        }
        // Return an empty MapperExpr.
        return MapperExpr {
            node: ExprNode::new(),
            vector_size,
            history_size: 1,
            history_pos: -1,
            input_history: vec![MapperSignalValue::F(0.0); vector_size.max(1) as usize],
            output_history: vec![MapperSignalValue::F(0.0); 1],
        };
    }

    let mut nodes = result.unwrap();

    if oldest_samps < -100.0 {
        // History reference too far back — bail out with empty result.
        return MapperExpr {
            node: ExprNode::new(),
            vector_size,
            history_size: 1,
            history_pos: -1,
            input_history: vec![MapperSignalValue::F(0.0); vector_size.max(1) as usize],
            output_history: vec![MapperSignalValue::F(0.0); 1],
        };
    }

    // Coerce the final output if necessary.
    let last_is_float = nodes.last().map(|n| n.is_float != 0).unwrap_or(false);
    if last_is_float && output_is_float == 0 {
        nodes.push(InternalNode {
            tok: Token {
                token_type: TokenType::ToInt32,
                value: None,
                int_value: None,
                var: None,
                op: None,
            },
            is_float: 0,
            history_index: 0,
            vector_index: 0,
        });
    } else if !last_is_float && output_is_float != 0 {
        nodes.push(InternalNode {
            tok: Token {
                token_type: TokenType::ToFloat,
                value: None,
                int_value: None,
                var: None,
                op: None,
            },
            is_float: 1,
            history_index: 0,
            vector_index: 0,
        });
    }

    // For vector_size > 1, disallow vector indexing (faked element-wise eval).
    if vector_size > 1 {
        for n in &nodes {
            if n.tok.token_type == TokenType::Var && n.vector_index > 0 {
                return MapperExpr {
                    node: ExprNode::new(),
                    vector_size,
                    history_size: 1,
                    history_pos: -1,
                    input_history: vec![
                        MapperSignalValue::F(0.0);
                        vector_size.max(1) as usize
                    ],
                    output_history: vec![MapperSignalValue::F(0.0); 1],
                };
            }
        }
    }

    let history_size = ((-oldest_samps).ceil() as i32) + 1;
    let vec_sz = vector_size.max(1) as usize;
    let hs = history_size.max(1) as usize;

    MapperExpr {
        node: vec_to_expr_node(&nodes),
        vector_size,
        history_size,
        history_pos: -1,
        input_history: vec![MapperSignalValue::F(0.0); vec_sz * hs],
        output_history: vec![MapperSignalValue::F(0.0); hs],
    }
}

// ---- Evaluation ----

fn mapper_expr_evaluate_internal(
    mapper: &mut MapperExpr,
    input_vector: Option<&[MapperSignalValue]>,
    output_default: &MapperSignalValue,
) -> MapperSignalValue {
    let _ = output_default;
    let mut stack: Vec<MapperSignalValue> = Vec::with_capacity(STACK_SIZE);

    if let Some(inv) = input_vector {
        if mapper.history_size > 0 {
            mapper.history_pos = (mapper.history_pos + 1).rem_euclid(mapper.history_size);
        }
        let pos = mapper.history_pos as usize;
        let vs = mapper.vector_size.max(1) as usize;
        for i in 0..vs.min(inv.len()) {
            let dst = pos * vs + i;
            if dst < mapper.input_history.len() {
                mapper.input_history[dst] = inv[i];
            }
        }
    }

    // Iterate via Arc chain.
    // We need to walk: first the head node (mapper.node), then mapper.node.next,
    // and so on until we hit None.
    let head = &mapper.node;
    let mut cursor: Option<&ExprNode> = Some(head);
    let mut chain_cursor: Option<&Arc<ExprNode>> = head.next.as_ref();

    while let Some(node) = cursor {
        match node.tok.token_type {
            TokenType::Int => {
                stack.push(MapperSignalValue::I32(node.tok.int_value.unwrap_or(0)));
            }
            TokenType::Float => {
                stack.push(MapperSignalValue::F(node.tok.value.unwrap_or(0.0)));
            }
            TokenType::Var => {
                if mapper.history_size <= 0 {
                    return MapperSignalValue::I32(0);
                }
                let idx = ((node.history_index + mapper.history_pos + mapper.history_size)
                    .rem_euclid(mapper.history_size)) as usize;
                let vs = mapper.vector_size.max(1) as usize;
                let v = match node.tok.var {
                    Some('x') => {
                        let off = idx * vs + node.vector_index as usize;
                        if off < mapper.input_history.len() {
                            mapper.input_history[off]
                        } else {
                            MapperSignalValue::I32(0)
                        }
                    }
                    Some('y') => {
                        if idx < mapper.output_history.len() {
                            mapper.output_history[idx]
                        } else {
                            MapperSignalValue::I32(0)
                        }
                    }
                    _ => return MapperSignalValue::I32(0),
                };
                stack.push(v);
            }
            TokenType::ToFloat => {
                if let Some(top) = stack.last_mut() {
                    let f = val_to_f32(top);
                    *top = MapperSignalValue::F(f);
                }
            }
            TokenType::ToInt32 => {
                if let Some(top) = stack.last_mut() {
                    let i = val_to_i32(top);
                    *top = MapperSignalValue::I32(i);
                }
            }
            TokenType::Op => {
                let right = stack.pop().unwrap_or(MapperSignalValue::I32(0));
                let left = stack.pop().unwrap_or(MapperSignalValue::I32(0));
                if node.is_float != 0 {
                    let l = val_to_f32(&left);
                    let r = val_to_f32(&right);
                    let v = match node.tok.op {
                        Some('+') => l + r,
                        Some('-') => l - r,
                        Some('*') => l * r,
                        Some('/') => l / r,
                        _ => return MapperSignalValue::I32(0),
                    };
                    stack.push(MapperSignalValue::F(v));
                } else {
                    let l = val_to_i32(&left);
                    let r = val_to_i32(&right);
                    let v = match node.tok.op {
                        Some('+') => l.wrapping_add(r),
                        Some('-') => l.wrapping_sub(r),
                        Some('*') => l.wrapping_mul(r),
                        Some('/') => {
                            if r == 0 {
                                0
                            } else {
                                l.wrapping_div(r)
                            }
                        }
                        _ => return MapperSignalValue::I32(0),
                    };
                    stack.push(MapperSignalValue::I32(v));
                }
            }
            TokenType::Func => {
                let idx = node.tok.int_value.unwrap_or(-1);
                let entry = match func_entry_by_index(idx) {
                    Some(e) => e,
                    None => return MapperSignalValue::I32(0),
                };
                match entry.arity {
                    0 => {
                        let f = (entry.func)(0.0, 0.0);
                        stack.push(MapperSignalValue::F(f));
                    }
                    1 => {
                        let r = stack.pop().unwrap_or(MapperSignalValue::F(0.0));
                        let rf = val_to_f32(&r);
                        let f = (entry.func)(rf, 0.0);
                        stack.push(MapperSignalValue::F(f));
                    }
                    2 => {
                        let r = stack.pop().unwrap_or(MapperSignalValue::F(0.0));
                        let l = stack.pop().unwrap_or(MapperSignalValue::F(0.0));
                        let lf = val_to_f32(&l);
                        let rf = val_to_f32(&r);
                        let f = (entry.func)(lf, rf);
                        stack.push(MapperSignalValue::F(f));
                    }
                    _ => return MapperSignalValue::I32(0),
                }
            }
            _ => return MapperSignalValue::I32(0),
        }

        // advance cursor
        if let Some(arc) = chain_cursor {
            cursor = Some(arc.as_ref());
            chain_cursor = arc.next.as_ref();
        } else {
            cursor = None;
        }
    }

    let result = stack.into_iter().next().unwrap_or(MapperSignalValue::I32(0));

    if input_vector.is_some() {
        let pos = mapper.history_pos as usize;
        if pos < mapper.output_history.len() {
            mapper.output_history[pos] = result;
        }
    }

    result
}

pub fn mapper_expr_evaluate<'a>(
    mapper: &mut MapperExpr,
    input: &'a MapperSignalValue,
) -> MapperSignalValue {
    // Build a small input vector from the single value (vector_size==1 is the
    // primary supported case). For vector_size > 1 we replicate the value.
    let vs = mapper.vector_size.max(1) as usize;
    let inv: Vec<MapperSignalValue> = vec![*input; vs];
    mapper_expr_evaluate_internal(mapper, Some(&inv), input)
}
