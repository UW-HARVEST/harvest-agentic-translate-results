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
            MapperSignalValue::F(v) => Some(*v),
            MapperSignalValue::I32(v) => Some(*v as f32),
        }
    }
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            MapperSignalValue::I32(v) => Some(*v),
            MapperSignalValue::F(v) => Some(*v as i32),
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
        m.insert("asin", FunctionEntry { name: "asin", arity: 1, func: |x, _| x.asin() });
        m.insert("acos", FunctionEntry { name: "acos", arity: 1, func: |x, _| x.acos() });
        m.insert("atan", FunctionEntry { name: "atan", arity: 1, func: |x, _| x.atan() });
        m.insert("atan2", FunctionEntry { name: "atan2", arity: 2, func: f32::atan2 });
        m.insert("sinh", FunctionEntry { name: "sinh", arity: 1, func: |x, _| x.sinh() });
        m.insert("cosh", FunctionEntry { name: "cosh", arity: 1, func: |x, _| x.cosh() });
        m.insert("tanh", FunctionEntry { name: "tanh", arity: 1, func: |x, _| x.tanh() });
        m.insert("logb", FunctionEntry { name: "logb", arity: 1, func: |x, _| {
            if x == 0.0 { f32::NEG_INFINITY } else { x.abs().log2().floor() }
        }});
        m.insert("exp2", FunctionEntry { name: "exp2", arity: 1, func: |x, _| x.exp2() });
        m.insert("log2", FunctionEntry { name: "log2", arity: 1, func: |x, _| x.log2() });
        m.insert("hypot", FunctionEntry { name: "hypot", arity: 2, func: f32::hypot });
        m.insert("cbrt", FunctionEntry { name: "cbrt", arity: 1, func: |x, _| x.cbrt() });
        m.insert("trunc", FunctionEntry { name: "trunc", arity: 1, func: |x, _| x.trunc() });
        m.insert("min", FunctionEntry { name: "min", arity: 2, func: minf });
        m.insert("max", FunctionEntry { name: "max", arity: 2, func: maxf });
        m.insert("pi", FunctionEntry { name: "pi", arity: 0, func: |_, _| pif() });
        m
    };

    // Stable index-based access. We store function indices in token's int_value field.
    static ref FUNCTION_LIST: Vec<(String, FunctionEntry)> = {
        let names = [
            "pow", "sin", "cos", "tan", "abs", "sqrt", "log", "log10", "exp",
            "floor", "round", "ceil", "asin", "acos", "atan", "atan2",
            "sinh", "cosh", "tanh", "logb", "exp2", "log2", "hypot", "cbrt",
            "trunc", "min", "max", "pi",
        ];
        names.iter().map(|n| (n.to_string(), *FUNCTION_TABLE.get(n).unwrap())).collect()
    };
}

fn func_index_lookup(name: &str) -> Option<i32> {
    FUNCTION_LIST.iter().position(|(n, _)| n == name).map(|i| i as i32)
}

fn func_by_index(idx: i32) -> Option<&'static FunctionEntry> {
    FUNCTION_LIST.get(idx as usize).map(|(_, e)| e)
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
fn function_lookup(s: &str) -> Option<&'static FunctionEntry> {
    FUNCTION_TABLE.get(s)
}

fn func_arity(name: &str) -> u32 {
    FUNCTION_TABLE.get(name).map(|e| e.arity).unwrap_or(0)
}

fn func_apply(name: &str, x: f32, y: f32) -> f32 {
    if let Some(e) = FUNCTION_TABLE.get(name) {
        (e.func)(x, y)
    } else {
        0.0
    }
}

fn expr_lex(s: Vec<&str>) -> Vec<Token> {
    // Concatenate all input pieces, lex into tokens
    let joined: String = s.into_iter().collect::<Vec<_>>().join("");
    let bytes: Vec<char> = joined.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let tok = lex_one(&bytes, &mut i);
        if let Some(t) = tok {
            let is_end = t.token_type == TokenType::End;
            tokens.push(t);
            if is_end {
                break;
            }
        } else {
            break;
        }
    }
    tokens
}

fn lex_one(s: &[char], pos: &mut usize) -> Option<Token> {
    // Skip whitespace
    while *pos < s.len() {
        let c = s[*pos];
        if c == ' ' || c == '\t' || c == '\r' || c == '\n' {
            *pos += 1;
        } else {
            break;
        }
    }
    if *pos >= s.len() {
        return Some(Token {
            token_type: TokenType::End,
            value: None,
            int_value: None,
            var: None,
            op: None,
        });
    }
    let c = s[*pos];
    let mut integer_found = false;
    let mut int_val: i32 = 0;

    if c.is_ascii_digit() {
        let start = *pos;
        while *pos < s.len() && s[*pos].is_ascii_digit() {
            *pos += 1;
        }
        let int_str: String = s[start..*pos].iter().collect();
        int_val = int_str.parse::<i32>().unwrap_or(0);
        integer_found = true;
        let nc = if *pos < s.len() { s[*pos] } else { '\0' };
        if nc != '.' {
            return Some(Token {
                token_type: TokenType::Int,
                value: None,
                int_value: Some(int_val),
                var: None,
                op: None,
            });
        }
    }

    let c = if *pos < s.len() { s[*pos] } else { '\0' };
    match c {
        '.' => {
            let start = *pos;
            *pos += 1;
            let nc = if *pos < s.len() { s[*pos] } else { '\0' };
            if !nc.is_ascii_digit() && integer_found {
                return Some(Token {
                    token_type: TokenType::Float,
                    value: Some(int_val as f32),
                    int_value: None,
                    var: None,
                    op: None,
                });
            }
            if !nc.is_ascii_digit() {
                return None;
            }
            while *pos < s.len() && s[*pos].is_ascii_digit() {
                *pos += 1;
            }
            let float_str: String = s[start..*pos].iter().collect();
            let frac: f64 = float_str.parse::<f64>().unwrap_or(0.0);
            Some(Token {
                token_type: TokenType::Float,
                value: Some((int_val as f64 + frac) as f32),
                int_value: None,
                var: None,
                op: None,
            })
        }
        '+' | '-' | '/' | '*' | '=' => {
            *pos += 1;
            Some(Token {
                token_type: TokenType::Op,
                value: None,
                int_value: None,
                var: None,
                op: Some(c),
            })
        }
        '(' => {
            *pos += 1;
            Some(Token {
                token_type: TokenType::OpenParen,
                value: None,
                int_value: None,
                var: None,
                op: None,
            })
        }
        ')' => {
            *pos += 1;
            Some(Token {
                token_type: TokenType::CloseParen,
                value: None,
                int_value: None,
                var: None,
                op: None,
            })
        }
        'x' | 'y' => {
            *pos += 1;
            Some(Token {
                token_type: TokenType::Var,
                value: None,
                int_value: None,
                var: Some(c),
                op: None,
            })
        }
        '[' => {
            *pos += 1;
            Some(Token {
                token_type: TokenType::OpenSquare,
                value: None,
                int_value: None,
                var: None,
                op: None,
            })
        }
        ']' => {
            *pos += 1;
            Some(Token {
                token_type: TokenType::CloseSquare,
                value: None,
                int_value: None,
                var: None,
                op: None,
            })
        }
        '{' => {
            *pos += 1;
            Some(Token {
                token_type: TokenType::OpenCurly,
                value: None,
                int_value: None,
                var: None,
                op: None,
            })
        }
        '}' => {
            *pos += 1;
            Some(Token {
                token_type: TokenType::CloseCurly,
                value: None,
                int_value: None,
                var: None,
                op: None,
            })
        }
        ',' => {
            *pos += 1;
            Some(Token {
                token_type: TokenType::Comma,
                value: None,
                int_value: None,
                var: None,
                op: None,
            })
        }
        '\0' => Some(Token {
            token_type: TokenType::End,
            value: None,
            int_value: None,
            var: None,
            op: None,
        }),
        _ => {
            if !c.is_ascii_alphabetic() {
                println!("unknown character '{}' in lexer", c);
                return None;
            }
            let start = *pos;
            while *pos < s.len() {
                let cc = s[*pos];
                if cc.is_ascii_alphabetic() || cc.is_ascii_digit() {
                    *pos += 1;
                } else {
                    break;
                }
            }
            let name: String = s[start..*pos].iter().collect();
            // Store the function name in `var` field (we re-use char field for index hack);
            // Since Token only has Option<char>, we need different storage. Let's encode
            // the function as a TokenType::Func and put name into a separate global lookup.
            // Instead, we will encode by storing the function entry's char as None, and
            // we'll separately track the function name in a side table.
            // But we don't want side tables, so instead we store via a thread-local map.
            // Actually we need the name later — use a workaround: store a marker by op
            // and track in a global. To simplify, we will store the function name in
            // the parser's NodeData not Token.
            // For the lexer we need to know the function. We'll attach the name through
            // a parallel mechanism: a special Token variant with var=first char of name
            // is not enough. Use a thread-local registry mapping a token id to name.
            register_func_name(&name);
            // Encode function index in int_value (-1 if unknown)
            let idx = func_index_lookup(&name).unwrap_or(-1);
            Some(Token {
                token_type: TokenType::Func,
                value: None,
                int_value: Some(idx),
                var: None,
                op: None,
            })
        }
    }
}

use std::cell::RefCell;
thread_local! {
    static FUNC_NAME_QUEUE: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

fn register_func_name(name: &str) {
    FUNC_NAME_QUEUE.with(|q| q.borrow_mut().push(name.to_string()));
}

fn pop_func_name() -> String {
    FUNC_NAME_QUEUE.with(|q| {
        let mut q = q.borrow_mut();
        if q.is_empty() {
            String::new()
        } else {
            q.remove(0)
        }
    })
}

fn clear_func_names() {
    FUNC_NAME_QUEUE.with(|q| q.borrow_mut().clear());
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
impl ExprNode {
    pub fn new() -> ExprNode {
        ExprNode {
            tok: Token {
                token_type: TokenType::End,
                value: None,
                int_value: None,
                var: None,
                op: None,
            },
            is_float: 0,
            history_index: 0,
            vector_index: 0,
            next: None,
        }
    }
    pub fn expr_free(&self) {
        // Rust handles freeing automatically via Arc drop
    }
}

fn printtoken(t: &Token) {
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
        TokenType::Func => print!("FUNC()"),
        TokenType::Comma => print!(","),
        TokenType::End => print!("END"),
        TokenType::ToFloat => print!("(float)"),
        TokenType::ToInt32 => print!("(int32)"),
    }
}

fn printexprnode(s: &str, list: &ExprNode) {
    print!("{}", s);
    let mut cur: Option<&ExprNode> = Some(list);
    while let Some(node) = cur {
        if node.is_float != 0
            && node.tok.token_type != TokenType::Float
            && node.tok.token_type != TokenType::ToFloat
        {
            print!(".");
        }
        printtoken(&node.tok);
        if node.tok.token_type == TokenType::Var {
            if node.history_index < 0 {
                print!("{{{}}}", node.history_index);
            }
            if node.vector_index > -1 {
                print!("[{}]", node.vector_index);
            }
        }
        cur = node.next.as_deref();
        if cur.is_some() {
            print!(" ");
        }
    }
}

fn printexpr(s: &str, list: &MapperExpr) {
    printexprnode(s, &list.node);
}

fn printstack(_stack: &stack_obj_t, _stack_size: i32) {
    // Debug only
}

fn collapse_expr_to_left(_plhs: &mut ExprNode, _constant_folding: i32) {
    // Implemented via internal arena in parser
}
// Internal arena-based representation for parsing.
#[derive(Clone)]
struct NodeData {
    tok: Token,
    is_float: i32,
    history_index: i32,
    vector_index: i32,
    next: Option<usize>,
    func_name: Option<String>,
}

struct Arena {
    nodes: Vec<NodeData>,
}

impl Arena {
    fn new() -> Self {
        Arena { nodes: Vec::new() }
    }
    fn alloc(&mut self, tok: Token, is_float: i32, func_name: Option<String>) -> usize {
        self.nodes.push(NodeData {
            tok,
            is_float,
            history_index: 0,
            vector_index: 0,
            next: None,
            func_name,
        });
        self.nodes.len() - 1
    }
    fn last_in_chain(&self, head: usize) -> usize {
        let mut cur = head;
        while let Some(n) = self.nodes[cur].next {
            cur = n;
        }
        cur
    }
    fn has_var(&self, head: usize) -> bool {
        let mut cur = Some(head);
        while let Some(idx) = cur {
            if self.nodes[idx].tok.token_type == TokenType::Var {
                return true;
            }
            cur = self.nodes[idx].next;
        }
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InternalState {
    YEqualY,
    YEqualEq,
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

#[derive(Clone, Copy, Debug)]
enum StackEntry {
    State(InternalState),
    Node(usize), // index into arena (head of chain)
}

// Apply the same logic as collapse_expr_to_left in C.
fn arena_collapse(arena: &mut Arena, lhs_head: usize, rhs_head: usize, constant_folding: bool) -> usize {
    let mut refvar = arena.has_var(lhs_head) || arena.has_var(rhs_head);
    let _ = refvar;
    refvar = arena.has_var(lhs_head) || arena.has_var(rhs_head);

    // Find lhs_last (index of last node in lhs chain)
    let mut lhs_prev: Option<usize> = None;
    let mut lhs_last = lhs_head;
    while let Some(n) = arena.nodes[lhs_last].next {
        lhs_prev = Some(lhs_last);
        lhs_last = n;
    }
    // Find rhs_last
    let mut rhs_last = rhs_head;
    while let Some(n) = arena.nodes[rhs_last].next {
        rhs_last = n;
    }

    let lhs_last_is_float = arena.nodes[lhs_last].is_float != 0;
    let rhs_last_is_float = arena.nodes[rhs_last].is_float != 0;
    let is_float = lhs_last_is_float || rhs_last_is_float;

    // Coerce types if disagree
    let mut new_lhs_head = lhs_head;
    let mut effective_lhs_last = lhs_last;
    let mut effective_lhs_prev = lhs_prev;
    let coerce_tok = Token {
        token_type: TokenType::ToFloat,
        value: None,
        int_value: None,
        var: None,
        op: None,
    };

    if lhs_last_is_float && !rhs_last_is_float {
        // append a TOFLOAT to rhs end
        let cidx = arena.alloc(coerce_tok, 1, None);
        arena.nodes[rhs_last].next = Some(cidx);
        // rhs_last = cidx; (no longer used after this)
    } else if !lhs_last_is_float && rhs_last_is_float {
        // insert a TOFLOAT BEFORE lhs_last:
        // i.e. we replace what previously pointed to lhs_last with a new node 'e',
        // and e.next = lhs_last; mark lhs_last.is_float = 1
        let cidx = arena.alloc(coerce_tok, 1, None);
        arena.nodes[cidx].next = Some(lhs_last);
        if let Some(p) = lhs_prev {
            arena.nodes[p].next = Some(cidx);
        } else {
            // lhs was a single node; replace head
            new_lhs_head = cidx;
        }
        // mark lhs_last as is_float = 1 (matching e->next->is_float = 1)
        arena.nodes[lhs_last].is_float = 1;
        // Now plhs_last (the slot pointing to lhs_last) is &cidx.next
        effective_lhs_prev = Some(cidx);
        effective_lhs_last = lhs_last;
    }

    // Insert rhs list before lhs_last (the trailing op):
    // rhs_last.next = lhs_last
    // *plhs_last (the slot that pointed to lhs_last) = rhs
    let _ = effective_lhs_last;
    // Find the actual rhs tail again (in case TOFLOAT was appended)
    let rhs_tail_now = arena.last_in_chain(rhs_head);
    arena.nodes[rhs_tail_now].next = Some(effective_lhs_last);
    if let Some(p) = effective_lhs_prev {
        arena.nodes[p].next = Some(rhs_head);
    } else {
        new_lhs_head = rhs_head;
    }

    // Constant folding: evaluate expression now if no variables
    if constant_folding && !refvar {
        // Build a temporary MapperExpr-like environment and evaluate.
        if let Some(v) = evaluate_arena_chain(arena, new_lhs_head, None, 0, 0, &[]) {
            // Replace head with single literal node (and free chain).
            arena.nodes[new_lhs_head].next = None;
            arena.nodes[new_lhs_head].is_float = if is_float { 1 } else { 0 };
            if is_float {
                arena.nodes[new_lhs_head].tok = Token {
                    token_type: TokenType::Float,
                    value: Some(v.f),
                    int_value: None,
                    var: None,
                    op: None,
                };
            } else {
                arena.nodes[new_lhs_head].tok = Token {
                    token_type: TokenType::Int,
                    value: None,
                    int_value: Some(v.i32),
                    var: None,
                    op: None,
                };
            }
            arena.nodes[new_lhs_head].func_name = None;
        }
    }

    new_lhs_head
}

#[derive(Clone, Copy)]
struct SignalUnion {
    f: f32,
    i32: i32,
}

fn evaluate_arena_chain(
    arena: &Arena,
    head: usize,
    input: Option<&MapperSignalValue>,
    history_pos: i32,
    history_size: i32,
    input_history: &[MapperSignalValue],
) -> Option<SignalUnion> {
    let _ = (input, history_pos, history_size, input_history);
    // For constant folding, no vars. We can use a stack.
    let mut stack: Vec<SignalUnion> = Vec::with_capacity(STACK_SIZE);
    let mut cur = Some(head);
    while let Some(idx) = cur {
        let node = &arena.nodes[idx];
        match node.tok.token_type {
            TokenType::Int => {
                stack.push(SignalUnion {
                    f: 0.0,
                    i32: node.tok.int_value.unwrap_or(0),
                });
            }
            TokenType::Float => {
                stack.push(SignalUnion {
                    f: node.tok.value.unwrap_or(0.0),
                    i32: 0,
                });
            }
            TokenType::ToFloat => {
                let top = stack.last_mut()?;
                top.f = top.i32 as f32;
            }
            TokenType::ToInt32 => {
                let top = stack.last_mut()?;
                top.i32 = top.f as i32;
            }
            TokenType::Op => {
                let right = stack.pop()?;
                let left = stack.pop()?;
                let op = node.tok.op?;
                if node.is_float != 0 {
                    let r = match op {
                        '+' => left.f + right.f,
                        '-' => left.f - right.f,
                        '*' => left.f * right.f,
                        '/' => left.f / right.f,
                        _ => return None,
                    };
                    stack.push(SignalUnion { f: r, i32: 0 });
                } else {
                    let r = match op {
                        '+' => left.i32.wrapping_add(right.i32),
                        '-' => left.i32.wrapping_sub(right.i32),
                        '*' => left.i32.wrapping_mul(right.i32),
                        '/' => {
                            if right.i32 == 0 {
                                return None;
                            } else {
                                left.i32 / right.i32
                            }
                        }
                        _ => return None,
                    };
                    stack.push(SignalUnion { f: 0.0, i32: r });
                }
            }
            TokenType::Func => {
                let idx = node.tok.int_value.unwrap_or(-1);
                let entry = match func_by_index(idx) {
                    Some(e) => e,
                    None => return None,
                };
                match entry.arity {
                    0 => {
                        let v = (entry.func)(0.0, 0.0);
                        stack.push(SignalUnion { f: v, i32: 0 });
                    }
                    1 => {
                        let r = stack.pop()?;
                        let v = (entry.func)(r.f, 0.0);
                        stack.push(SignalUnion { f: v, i32: 0 });
                    }
                    2 => {
                        let right = stack.pop()?;
                        let left = stack.pop()?;
                        let v = (entry.func)(left.f, right.f);
                        stack.push(SignalUnion { f: v, i32: 0 });
                    }
                    _ => return None,
                }
            }
            _ => return None,
        }
        cur = node.next;
    }
    if stack.is_empty() {
        None
    } else {
        Some(stack[0])
    }
}

// Convert an arena-based chain into an Arc<ExprNode> linked list (in reverse).
fn arena_to_arc_chain(arena: &Arena, head: usize) -> Option<Arc<ExprNode>> {
    // Collect all nodes in order.
    let mut indices = Vec::new();
    let mut cur = Some(head);
    while let Some(idx) = cur {
        indices.push(idx);
        cur = arena.nodes[idx].next;
    }
    // Build from the tail back.
    let mut next: Option<Arc<ExprNode>> = None;
    for &idx in indices.iter().rev() {
        let n = &arena.nodes[idx];
        let mut tok = n.tok;
        // Encode function name into Token's `var` field is impossible, so we
        // need to keep func name lookup somewhere — store it in a side map keyed
        // by the Arc identity. We'll handle this by attaching name via a separate
        // global registry keyed by ExprNode pointer. Simpler: store the func name
        // in token's var field as a single placeholder char and look up via a
        // node-pointer registry.
        let _ = tok;
        let func_name = n.func_name.clone();
        let node = Arc::new(ExprNode {
            tok: n.tok,
            is_float: n.is_float,
            history_index: n.history_index,
            vector_index: n.vector_index,
            next: next.clone(),
        });
        // Register func name (if any) with this node's pointer identity.
        if let Some(fname) = func_name {
            register_node_func(Arc::as_ptr(&node) as usize, fname);
        }
        next = Some(node);
    }
    next
}

thread_local! {
    static NODE_FUNC_NAMES: RefCell<HashMap<usize, String>> = RefCell::new(HashMap::new());
}

fn register_node_func(ptr: usize, name: String) {
    NODE_FUNC_NAMES.with(|m| {
        m.borrow_mut().insert(ptr, name);
    });
}

fn lookup_node_func(node: &ExprNode) -> Option<String> {
    let ptr = node as *const ExprNode as usize;
    NODE_FUNC_NAMES.with(|m| m.borrow().get(&ptr).cloned())
}

pub fn mapper_expr_new_from_string(
    s: &str,
    input_is_float: i32,
    _output_is_float: i32,
    vector_size: i32,
) -> MapperExpr {
    clear_func_names();
    let tokens = expr_lex(vec![s]);
    let mut tok_iter = TokenIter::new(tokens);
    let mut arena = Arena::new();
    let mut stack: Vec<StackEntry> = Vec::with_capacity(STACK_SIZE);

    stack.push(StackEntry::State(InternalState::Expr));
    stack.push(StackEntry::State(InternalState::YEqualEq));
    stack.push(StackEntry::State(InternalState::YEqualY));

    let mut next_token = true;
    let mut var_allowed = true;
    let mut oldest_samps: f32 = 0.0;
    let mut result_head: Option<usize> = None;
    let mut error: Option<&'static str> = None;
    let mut cur_tok = Token {
        token_type: TokenType::End,
        value: None,
        int_value: None,
        var: None,
        op: None,
    };

    while !stack.is_empty() {
        if next_token {
            cur_tok = tok_iter.next();
            next_token = false;
        }

        // If top is a node, do "tail logic"
        let top_idx = stack.len() - 1;
        match stack[top_idx] {
            StackEntry::Node(node_head) => {
                if top_idx == 0 {
                    result_head = Some(node_head);
                    break;
                }
                let below = stack[top_idx - 1];
                if let StackEntry::State(state) = below {
                    if top_idx >= 2 {
                        if let StackEntry::Node(below2) = stack[top_idx - 2] {
                            match state {
                                InternalState::ExprRight
                                | InternalState::TermRight
                                | InternalState::CloseParen => {
                                    let new_head = arena_collapse(&mut arena, below2, node_head, true);
                                    stack[top_idx - 2] = StackEntry::Node(new_head);
                                    stack.pop(); // remove top
                                }
                                InternalState::CloseHistIndex => {
                                    // expected VAR two-down
                                    let val_node = &arena.nodes[node_head];
                                    let hi = match val_node.tok.token_type {
                                        TokenType::Float => val_node.tok.value.unwrap_or(0.0) as i32,
                                        TokenType::Int => val_node.tok.int_value.unwrap_or(0),
                                        _ => 0,
                                    };
                                    arena.nodes[below2].history_index = hi;
                                    if (oldest_samps as i32) > hi {
                                        oldest_samps = hi as f32;
                                    }
                                    stack.pop();
                                }
                                InternalState::CloseVectIndex => {
                                    let val_node = &arena.nodes[node_head];
                                    let vi = match val_node.tok.token_type {
                                        TokenType::Float => val_node.tok.value.unwrap_or(0.0) as i32,
                                        TokenType::Int => val_node.tok.int_value.unwrap_or(0),
                                        _ => 0,
                                    };
                                    arena.nodes[below2].vector_index = vi;
                                    if vi > 0 {
                                        error = Some("Vector indexing not yet implemented.");
                                        break;
                                    }
                                    if vi < 0 || vi >= vector_size {
                                        error = Some("Vector index outside input size.");
                                        break;
                                    }
                                    stack.pop();
                                }
                                _ => {
                                    // swap node down: not applicable when below2 is also Node
                                    let tmp = stack[top_idx - 1];
                                    stack[top_idx - 1] = stack[top_idx];
                                    stack[top_idx] = tmp;
                                }
                            }
                        } else {
                            // swap node down
                            let tmp = stack[top_idx - 1];
                            stack[top_idx - 1] = stack[top_idx];
                            stack[top_idx] = tmp;
                        }
                    } else {
                        // top_idx == 1, below is state but no top-2; swap
                        let tmp = stack[top_idx - 1];
                        stack[top_idx - 1] = stack[top_idx];
                        stack[top_idx] = tmp;
                    }
                } else {
                    // below is also a node — shouldn't happen often.
                }
                continue;
            }
            StackEntry::State(state) => {
                match state {
                    InternalState::YEqualY => {
                        if cur_tok.token_type == TokenType::Var && cur_tok.var == Some('y') {
                            stack.pop();
                        } else {
                            error = Some("Error in y= prefix.");
                            break;
                        }
                        next_token = true;
                    }
                    InternalState::YEqualEq => {
                        if cur_tok.token_type == TokenType::Op && cur_tok.op == Some('=') {
                            stack.pop();
                        } else {
                            error = Some("Error in y= prefix.");
                            break;
                        }
                        next_token = true;
                    }
                    InternalState::Expr => {
                        stack.pop();
                        stack.push(StackEntry::State(InternalState::ExprRight));
                        stack.push(StackEntry::State(InternalState::Term));
                    }
                    InternalState::ExprRight => {
                        if cur_tok.token_type == TokenType::Op {
                            stack.pop();
                            let op = cur_tok.op.unwrap_or(' ');
                            if op == '+' || op == '-' {
                                // APPEND_OP
                                if let Some(StackEntry::Node(head)) = stack.last().copied() {
                                    let last = arena.last_in_chain(head);
                                    let is_float_prev = arena.nodes[last].is_float;
                                    let new_idx = arena.alloc(cur_tok, 0, None);
                                    arena.nodes[new_idx].is_float = is_float_prev;
                                    arena.nodes[last].next = Some(new_idx);
                                }
                                stack.push(StackEntry::State(InternalState::Expr));
                                next_token = true;
                            }
                        } else {
                            stack.pop();
                        }
                    }
                    InternalState::Term => {
                        stack.pop();
                        stack.push(StackEntry::State(InternalState::TermRight));
                        stack.push(StackEntry::State(InternalState::Value));
                    }
                    InternalState::TermRight => {
                        if cur_tok.token_type == TokenType::Op {
                            stack.pop();
                            let op = cur_tok.op.unwrap_or(' ');
                            if op == '*' || op == '/' {
                                if let Some(StackEntry::Node(head)) = stack.last().copied() {
                                    let last = arena.last_in_chain(head);
                                    let is_float_prev = arena.nodes[last].is_float;
                                    let new_idx = arena.alloc(cur_tok, 0, None);
                                    arena.nodes[new_idx].is_float = is_float_prev;
                                    arena.nodes[last].next = Some(new_idx);
                                }
                                stack.push(StackEntry::State(InternalState::Term));
                                next_token = true;
                            }
                        } else {
                            stack.pop();
                        }
                    }
                    InternalState::Value => {
                        match cur_tok.token_type {
                            TokenType::Int => {
                                stack.pop();
                                let idx = arena.alloc(cur_tok, 0, None);
                                stack.push(StackEntry::Node(idx));
                                next_token = true;
                            }
                            TokenType::Float => {
                                stack.pop();
                                let idx = arena.alloc(cur_tok, 1, None);
                                stack.push(StackEntry::Node(idx));
                                next_token = true;
                            }
                            TokenType::Var => {
                                if var_allowed {
                                    stack.pop();
                                    let idx = arena.alloc(cur_tok, input_is_float, None);
                                    stack.push(StackEntry::Node(idx));
                                    stack.push(StackEntry::State(InternalState::VarRight));
                                    next_token = true;
                                } else {
                                    error = Some("Unexpected variable reference.");
                                    break;
                                }
                            }
                            TokenType::OpenParen => {
                                stack.pop();
                                stack.push(StackEntry::State(InternalState::CloseParen));
                                stack.push(StackEntry::State(InternalState::Expr));
                                next_token = true;
                            }
                            TokenType::Func => {
                                stack.pop();
                                let _ = pop_func_name();
                                let func_idx = cur_tok.int_value.unwrap_or(-1);
                                let entry = func_by_index(func_idx);
                                if entry.is_none() {
                                    error = Some("Unknown function.");
                                    break;
                                }
                                let arity = entry.unwrap().arity;
                                let idx = arena.alloc(cur_tok, 1, None);
                                stack.push(StackEntry::Node(idx));
                                if arity > 0 {
                                    stack.push(StackEntry::State(InternalState::CloseParen));
                                    stack.push(StackEntry::State(InternalState::Expr));
                                    for _ in 1..arity {
                                        stack.push(StackEntry::State(InternalState::Comma));
                                        stack.push(StackEntry::State(InternalState::Expr));
                                    }
                                    stack.push(StackEntry::State(InternalState::OpenParen));
                                }
                                next_token = true;
                            }
                            TokenType::Op if cur_tok.op == Some('-') => {
                                stack.pop();
                                stack.push(StackEntry::State(InternalState::Negate));
                                stack.push(StackEntry::State(InternalState::Value));
                                next_token = true;
                            }
                            _ => {
                                error = Some("Expected value.");
                                break;
                            }
                        }
                    }
                    InternalState::Negate => {
                        stack.pop();
                        if let Some(StackEntry::Node(head)) = stack.last().copied() {
                            // create '0' node, then '-' op, then collapse with head
                            let zero_tok = Token {
                                token_type: TokenType::Int,
                                value: None,
                                int_value: Some(0),
                                var: None,
                                op: None,
                            };
                            let zero_idx = arena.alloc(zero_tok, 0, None);
                            let minus_tok = Token {
                                token_type: TokenType::Op,
                                value: None,
                                int_value: None,
                                var: None,
                                op: Some('-'),
                            };
                            let minus_idx = arena.alloc(minus_tok, 0, None);
                            arena.nodes[zero_idx].next = Some(minus_idx);
                            let new_head = arena_collapse(&mut arena, zero_idx, head, true);
                            *stack.last_mut().unwrap() = StackEntry::Node(new_head);
                        } else {
                            error = Some("Expected to negate an expression.");
                            break;
                        }
                    }
                    InternalState::VarRight => {
                        if cur_tok.token_type == TokenType::OpenSquare {
                            stack.pop();
                            stack.push(StackEntry::State(InternalState::VarVectIndex));
                        } else if cur_tok.token_type == TokenType::OpenCurly {
                            stack.pop();
                            stack.push(StackEntry::State(InternalState::VarHistIndex));
                        } else {
                            stack.pop();
                        }
                    }
                    InternalState::VarVectIndex => {
                        stack.pop();
                        if cur_tok.token_type == TokenType::OpenSquare {
                            var_allowed = false;
                            stack.push(StackEntry::State(InternalState::CloseVectIndex));
                            stack.push(StackEntry::State(InternalState::Expr));
                            next_token = true;
                        }
                    }
                    InternalState::VarHistIndex => {
                        stack.pop();
                        if cur_tok.token_type == TokenType::OpenCurly {
                            var_allowed = false;
                            stack.push(StackEntry::State(InternalState::CloseHistIndex));
                            stack.push(StackEntry::State(InternalState::Expr));
                            next_token = true;
                        }
                    }
                    InternalState::CloseVectIndex => {
                        if cur_tok.token_type == TokenType::CloseSquare {
                            var_allowed = true;
                            stack.pop();
                            stack.push(StackEntry::State(InternalState::VarHistIndex));
                            next_token = true;
                        } else {
                            error = Some("Expected ']'.");
                            break;
                        }
                    }
                    InternalState::CloseHistIndex => {
                        if cur_tok.token_type == TokenType::CloseCurly {
                            var_allowed = true;
                            stack.pop();
                            stack.push(StackEntry::State(InternalState::VarVectIndex));
                            next_token = true;
                        } else {
                            error = Some("Expected '}'.");
                            break;
                        }
                    }
                    InternalState::CloseParen => {
                        if cur_tok.token_type == TokenType::CloseParen {
                            stack.pop();
                            next_token = true;
                        } else {
                            error = Some("Expected ')'.");
                            break;
                        }
                    }
                    InternalState::Comma => {
                        if cur_tok.token_type == TokenType::Comma {
                            stack.pop();
                            // find previous node on stack starting from top-1
                            let mut found: Option<usize> = None;
                            if stack.len() >= 2 {
                                for i in (0..stack.len() - 1).rev() {
                                    if let StackEntry::Node(_) = stack[i] {
                                        found = Some(i);
                                        break;
                                    }
                                }
                            }
                            if let Some(prev_i) = found {
                                if let (StackEntry::Node(prev_head), StackEntry::Node(top_head)) =
                                    (stack[prev_i], stack[stack.len() - 1])
                                {
                                    let new_head = arena_collapse(&mut arena, prev_head, top_head, false);
                                    stack[prev_i] = StackEntry::Node(new_head);
                                    stack.pop();
                                }
                            }
                            next_token = true;
                        } else {
                            error = Some("Expected ','.");
                            break;
                        }
                    }
                    InternalState::OpenParen => {
                        if cur_tok.token_type == TokenType::OpenParen {
                            stack.pop();
                            next_token = true;
                        } else {
                            error = Some("Expected '('.");
                            break;
                        }
                    }
                    InternalState::End => {
                        if cur_tok.token_type == TokenType::End {
                            stack.pop();
                        } else {
                            error = Some("Expected END.");
                            break;
                        }
                    }
                }
            }
        }
    }

    if let Some(msg) = error {
        println!("{}", msg);
        // return a default empty expression
        return MapperExpr {
            node: ExprNode::new(),
            vector_size,
            history_size: 1,
            history_pos: -1,
            input_history: Vec::new(),
            output_history: Vec::new(),
        };
    }

    let head = match result_head {
        Some(h) => h,
        None => {
            return MapperExpr {
                node: ExprNode::new(),
                vector_size,
                history_size: 1,
                history_pos: -1,
                input_history: Vec::new(),
                output_history: Vec::new(),
            };
        }
    };

    if oldest_samps < -100.0 {
        return MapperExpr {
            node: ExprNode::new(),
            vector_size,
            history_size: 1,
            history_pos: -1,
            input_history: Vec::new(),
            output_history: Vec::new(),
        };
    }

    // Coerce the final output if necessary
    let last_idx = arena.last_in_chain(head);
    let last_is_float = arena.nodes[last_idx].is_float != 0;
    if last_is_float && _output_is_float == 0 {
        let coerce = Token {
            token_type: TokenType::ToInt32,
            value: None,
            int_value: None,
            var: None,
            op: None,
        };
        let cidx = arena.alloc(coerce, 0, None);
        arena.nodes[last_idx].next = Some(cidx);
    } else if !last_is_float && _output_is_float != 0 {
        let coerce = Token {
            token_type: TokenType::ToFloat,
            value: None,
            int_value: None,
            var: None,
            op: None,
        };
        let cidx = arena.alloc(coerce, 1, None);
        arena.nodes[last_idx].next = Some(cidx);
    }

    // Special case: vector_size > 1, disallow vector indexing
    if vector_size > 1 {
        let mut cur = Some(head);
        while let Some(idx) = cur {
            let n = &arena.nodes[idx];
            if n.tok.token_type == TokenType::Var && n.vector_index > 0 {
                return MapperExpr {
                    node: ExprNode::new(),
                    vector_size,
                    history_size: 1,
                    history_pos: -1,
                    input_history: Vec::new(),
                    output_history: Vec::new(),
                };
            }
            cur = n.next;
        }
    }

    let history_size = ((-oldest_samps).ceil() as i32) + 1;
    let arc_chain = arena_to_arc_chain(&arena, head);

    // The MapperExpr.node is owned. We need to construct it from arc_chain.
    // Since MapperExpr stores ExprNode by value (not Arc), we'll clone the head's content.
    let root_node = match arc_chain {
        Some(arc) => clone_expr_node_from_arc(&arc),
        None => ExprNode::new(),
    };

    let total = (vector_size as usize) * (history_size as usize);
    let input_history = vec![MapperSignalValue::I32(0); total.max(1)];
    let output_history = vec![MapperSignalValue::I32(0); history_size as usize];

    MapperExpr {
        node: root_node,
        vector_size,
        history_size,
        history_pos: -1,
        input_history,
        output_history,
    }
}

fn clone_expr_node_from_arc(arc: &Arc<ExprNode>) -> ExprNode {
    // Re-register func name for the new pointer if any.
    let fname = lookup_node_func(arc);
    let next = arc.next.clone();
    let new = ExprNode {
        tok: arc.tok,
        is_float: arc.is_float,
        history_index: arc.history_index,
        vector_index: arc.vector_index,
        next,
    };
    if let Some(n) = fname {
        register_node_func(&new as *const ExprNode as usize, n);
    }
    new
}

struct TokenIter {
    tokens: Vec<Token>,
    idx: usize,
}

impl TokenIter {
    fn new(tokens: Vec<Token>) -> Self {
        TokenIter { tokens, idx: 0 }
    }
    fn next(&mut self) -> Token {
        if self.idx < self.tokens.len() {
            let t = self.tokens[self.idx];
            self.idx += 1;
            t
        } else {
            Token {
                token_type: TokenType::End,
                value: None,
                int_value: None,
                var: None,
                op: None,
            }
        }
    }
}

pub fn mapper_expr_evaluate<'a>(
    mapper: &mut MapperExpr,
    input: &'a MapperSignalValue,
) -> MapperSignalValue {
    let history_size = mapper.history_size.max(1);
    mapper.history_pos = (mapper.history_pos + 1).rem_euclid(history_size);
    let pos = mapper.history_pos as usize;
    let vsize = mapper.vector_size.max(1) as usize;
    // Store input at history position (vector_size assumed 1 in scope)
    let idx = pos * vsize;
    if mapper.input_history.len() <= idx {
        mapper.input_history.resize(idx + vsize, MapperSignalValue::I32(0));
    }
    mapper.input_history[idx] = *input;

    // Evaluate
    let mut stack: Vec<SignalUnion> = Vec::with_capacity(STACK_SIZE);
    eval_node(&mapper.node, mapper, &mut stack);

    let result = if stack.is_empty() {
        SignalUnion { f: 0.0, i32: 0 }
    } else {
        stack[0]
    };

    // Store output at history pos
    // Determine result type from final node.
    let last_is_float = chain_last_is_float(&mapper.node);
    let out = if last_is_float {
        MapperSignalValue::F(result.f)
    } else {
        MapperSignalValue::I32(result.i32)
    };
    if mapper.output_history.len() <= pos {
        mapper.output_history.resize(pos + 1, MapperSignalValue::I32(0));
    }
    mapper.output_history[pos] = out;
    out
}

fn chain_last_is_float(head: &ExprNode) -> bool {
    let mut cur: &ExprNode = head;
    loop {
        match &cur.next {
            Some(n) => cur = n.as_ref(),
            None => break,
        }
    }
    cur.is_float != 0
}

fn eval_node(head: &ExprNode, mapper: &MapperExpr, stack: &mut Vec<SignalUnion>) {
    let mut cur_arc: Option<Arc<ExprNode>> = None;
    // Walk via reference first node, then via arc
    let mut cur_ref: Option<&ExprNode> = Some(head);
    loop {
        let node = match cur_ref {
            Some(n) => n,
            None => break,
        };
        match node.tok.token_type {
            TokenType::Int => {
                stack.push(SignalUnion {
                    f: 0.0,
                    i32: node.tok.int_value.unwrap_or(0),
                });
            }
            TokenType::Float => {
                stack.push(SignalUnion {
                    f: node.tok.value.unwrap_or(0.0),
                    i32: 0,
                });
            }
            TokenType::Var => {
                let idx = (node.history_index + mapper.history_pos + mapper.history_size)
                    .rem_euclid(mapper.history_size.max(1));
                let idx = idx as usize;
                let v = match node.tok.var {
                    Some('x') => {
                        let i = idx * (mapper.vector_size.max(1) as usize) + node.vector_index as usize;
                        if i < mapper.input_history.len() {
                            mapper.input_history[i]
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
                    _ => MapperSignalValue::I32(0),
                };
                let su = match v {
                    MapperSignalValue::F(f) => SignalUnion { f, i32: 0 },
                    MapperSignalValue::I32(i) => SignalUnion { f: 0.0, i32: i },
                };
                stack.push(su);
            }
            TokenType::ToFloat => {
                if let Some(top) = stack.last_mut() {
                    top.f = top.i32 as f32;
                }
            }
            TokenType::ToInt32 => {
                if let Some(top) = stack.last_mut() {
                    top.i32 = top.f as i32;
                }
            }
            TokenType::Op => {
                let right = stack.pop().unwrap_or(SignalUnion { f: 0.0, i32: 0 });
                let left = stack.pop().unwrap_or(SignalUnion { f: 0.0, i32: 0 });
                let op = node.tok.op.unwrap_or(' ');
                if node.is_float != 0 {
                    let r = match op {
                        '+' => left.f + right.f,
                        '-' => left.f - right.f,
                        '*' => left.f * right.f,
                        '/' => left.f / right.f,
                        _ => 0.0,
                    };
                    stack.push(SignalUnion { f: r, i32: 0 });
                } else {
                    let r = match op {
                        '+' => left.i32.wrapping_add(right.i32),
                        '-' => left.i32.wrapping_sub(right.i32),
                        '*' => left.i32.wrapping_mul(right.i32),
                        '/' => {
                            if right.i32 == 0 {
                                0
                            } else {
                                left.i32 / right.i32
                            }
                        }
                        _ => 0,
                    };
                    stack.push(SignalUnion { f: 0.0, i32: r });
                }
            }
            TokenType::Func => {
                let idx = node.tok.int_value.unwrap_or(-1);
                if let Some(entry) = func_by_index(idx) {
                    match entry.arity {
                        0 => {
                            let v = (entry.func)(0.0, 0.0);
                            stack.push(SignalUnion { f: v, i32: 0 });
                        }
                        1 => {
                            let r = stack.pop().unwrap_or(SignalUnion { f: 0.0, i32: 0 });
                            let v = (entry.func)(r.f, 0.0);
                            stack.push(SignalUnion { f: v, i32: 0 });
                        }
                        2 => {
                            let right = stack.pop().unwrap_or(SignalUnion { f: 0.0, i32: 0 });
                            let left = stack.pop().unwrap_or(SignalUnion { f: 0.0, i32: 0 });
                            let v = (entry.func)(left.f, right.f);
                            stack.push(SignalUnion { f: v, i32: 0 });
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        // Advance
        match &node.next {
            Some(arc) => {
                cur_arc = Some(arc.clone());
                cur_ref = cur_arc.as_deref();
            }
            None => {
                cur_ref = None;
            }
        }
    }
}