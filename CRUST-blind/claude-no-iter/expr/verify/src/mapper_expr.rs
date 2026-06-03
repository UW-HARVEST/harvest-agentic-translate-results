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
const TRACING: bool = false;
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
    static ref FUNCTION_LIST: Vec<FunctionEntry> = vec![
        FunctionEntry { name: "pow", arity: 2, func: f32::powf },
        FunctionEntry { name: "sin", arity: 1, func: |x, _| x.sin() },
        FunctionEntry { name: "cos", arity: 1, func: |x, _| x.cos() },
        FunctionEntry { name: "tan", arity: 1, func: |x, _| x.tan() },
        FunctionEntry { name: "abs", arity: 1, func: |x, _| x.abs() },
        FunctionEntry { name: "sqrt", arity: 1, func: |x, _| x.sqrt() },
        FunctionEntry { name: "log", arity: 1, func: |x, _| x.ln() },
        FunctionEntry { name: "log10", arity: 1, func: |x, _| x.log10() },
        FunctionEntry { name: "exp", arity: 1, func: |x, _| x.exp() },
        FunctionEntry { name: "floor", arity: 1, func: |x, _| x.floor() },
        FunctionEntry { name: "round", arity: 1, func: |x, _| x.round() },
        FunctionEntry { name: "ceil", arity: 1, func: |x, _| x.ceil() },
        FunctionEntry { name: "asin", arity: 1, func: |x, _| x.asin() },
        FunctionEntry { name: "acos", arity: 1, func: |x, _| x.acos() },
        FunctionEntry { name: "atan", arity: 1, func: |x, _| x.atan() },
        FunctionEntry { name: "atan2", arity: 2, func: f32::atan2 },
        FunctionEntry { name: "sinh", arity: 1, func: |x, _| x.sinh() },
        FunctionEntry { name: "cosh", arity: 1, func: |x, _| x.cosh() },
        FunctionEntry { name: "tanh", arity: 1, func: |x, _| x.tanh() },
        FunctionEntry { name: "logb", arity: 1, func: |x, _| x.abs().log2().floor() },
        FunctionEntry { name: "exp2", arity: 1, func: |x, _| x.exp2() },
        FunctionEntry { name: "log2", arity: 1, func: |x, _| x.log2() },
        FunctionEntry { name: "hypot", arity: 2, func: f32::hypot },
        FunctionEntry { name: "cbrt", arity: 1, func: |x, _| x.cbrt() },
        FunctionEntry { name: "trunc", arity: 1, func: |x, _| x.trunc() },
        FunctionEntry { name: "min", arity: 2, func: minf },
        FunctionEntry { name: "max", arity: 2, func: maxf },
        FunctionEntry { name: "pi", arity: 0, func: |_, _| pif() },
    ];
}
fn function_index_by_name(s: &str) -> Option<usize> {
    FUNCTION_LIST.iter().position(|e| e.name == s)
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
    FUNCTION_LIST.iter().find(|e| e.name == s)
}
fn make_token(token_type: TokenType) -> Token {
    Token {
        token_type,
        value: None,
        int_value: None,
        var: None,
        op: None,
    }
}
fn expr_lex(s: Vec<&str>) -> Vec<Token> {
    let joined: String = s.concat();
    let chars: Vec<char> = joined.chars().collect();
    let mut tokens: Vec<Token> = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        // skip whitespace
        if c == ' ' || c == '\t' || c == '\r' || c == '\n' {
            i += 1;
            continue;
        }
        let mut integer_found = false;
        let mut int_val: i32 = 0;
        let mut start = i;
        if c.is_ascii_digit() {
            start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let num_str: String = chars[start..i].iter().collect();
            int_val = num_str.parse::<i32>().unwrap_or(0);
            integer_found = true;
            if i >= chars.len() || chars[i] != '.' {
                let mut t = make_token(TokenType::Int);
                t.int_value = Some(int_val);
                tokens.push(t);
                continue;
            }
        }
        let cc = if i < chars.len() { chars[i] } else { '\0' };
        match cc {
            '.' => {
                let dot_start = i;
                i += 1;
                let mut has_frac = false;
                let frac_start = i;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                    has_frac = true;
                }
                if !has_frac && integer_found {
                    let mut t = make_token(TokenType::Float);
                    t.value = Some(int_val as f32);
                    tokens.push(t);
                    continue;
                }
                if !has_frac {
                    // unknown - skip
                    let _ = dot_start;
                    let _ = frac_start;
                    continue;
                }
                let frac_str: String = chars[dot_start..i].iter().collect();
                let frac_val: f64 = frac_str.parse::<f64>().unwrap_or(0.0);
                let mut t = make_token(TokenType::Float);
                t.value = Some(int_val as f32 + frac_val as f32);
                tokens.push(t);
            }
            '+' | '-' | '/' | '*' | '=' => {
                let mut t = make_token(TokenType::Op);
                t.op = Some(cc);
                tokens.push(t);
                i += 1;
            }
            '(' => {
                tokens.push(make_token(TokenType::OpenParen));
                i += 1;
            }
            ')' => {
                tokens.push(make_token(TokenType::CloseParen));
                i += 1;
            }
            'x' | 'y' => {
                // 'x' and 'y' are variables only if they are not part of a longer identifier
                // Look ahead: if next char is alpha or digit, then this is a function name.
                let next = if i + 1 < chars.len() { chars[i + 1] } else { '\0' };
                if next.is_ascii_alphanumeric() {
                    // treat as function identifier
                    start = i;
                    while i < chars.len() && (chars[i].is_ascii_alphabetic() || chars[i].is_ascii_digit()) {
                        i += 1;
                    }
                    let name: String = chars[start..i].iter().collect();
                    let mut t = make_token(TokenType::Func);
                    let idx = function_index_by_name(&name);
                    t.int_value = Some(match idx {
                        Some(v) => v as i32,
                        None => -1,
                    });
                    tokens.push(t);
                } else {
                    let mut t = make_token(TokenType::Var);
                    t.var = Some(cc);
                    tokens.push(t);
                    i += 1;
                }
            }
            '[' => {
                tokens.push(make_token(TokenType::OpenSquare));
                i += 1;
            }
            ']' => {
                tokens.push(make_token(TokenType::CloseSquare));
                i += 1;
            }
            '{' => {
                tokens.push(make_token(TokenType::OpenCurly));
                i += 1;
            }
            '}' => {
                tokens.push(make_token(TokenType::CloseCurly));
                i += 1;
            }
            ',' => {
                tokens.push(make_token(TokenType::Comma));
                i += 1;
            }
            _ => {
                if cc.is_ascii_alphabetic() {
                    start = i;
                    while i < chars.len() && (chars[i].is_ascii_alphabetic() || chars[i].is_ascii_digit()) {
                        i += 1;
                    }
                    let name: String = chars[start..i].iter().collect();
                    let mut t = make_token(TokenType::Func);
                    let idx = function_index_by_name(&name);
                    t.int_value = Some(match idx {
                        Some(v) => v as i32,
                        None => -1,
                    });
                    tokens.push(t);
                } else {
                    println!("unknown character '{}' in lexer", cc);
                    i += 1;
                }
            }
        }
    }
    tokens.push(make_token(TokenType::End));
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
impl ExprNode {
    pub fn new() -> ExprNode {
        ExprNode {
            tok: make_token(TokenType::End),
            is_float: 0,
            history_index: 0,
            vector_index: 0,
            next: None,
        }
    }
    pub fn expr_free(&self) {
        // In Rust, dropping the Arc chain handles cleanup automatically.
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
        TokenType::Func => {
            let idx = t.int_value.unwrap_or(-1);
            if idx >= 0 && (idx as usize) < FUNCTION_LIST.len() {
                print!("FUNC({})", FUNCTION_LIST[idx as usize].name);
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
fn printexprnode(s: &str, list: &ExprNode) {
    print!("{}", s);
    let mut cur: Option<&ExprNode> = Some(list);
    let mut first = true;
    while let Some(n) = cur {
        if !first {
            print!(" ");
        }
        first = false;
        if n.is_float != 0
            && n.tok.token_type != TokenType::Float
            && n.tok.token_type != TokenType::ToFloat
        {
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
        cur = n.next.as_deref();
    }
}
fn printexpr(s: &str, list: &MapperExpr) {
    printexprnode(s, &list.node);
}
fn printstack(_stack: &stack_obj_t, _stack_size: i32) {
    // Diagnostic helper - not used in the main flow.
}
fn collapse_expr_to_left(_plhs: &mut ExprNode, _constant_folding: i32) {
    // Stub for the public-facing variant. The internal parser uses
    // `collapse_intermediate_to_left` on Vec<IntermediateNode>.
}
// Intermediate node used during parsing - a flat Vec we can mutate.
#[derive(Clone, Debug)]
struct IntermediateNode {
    tok: Token,
    is_float: i32,
    history_index: i32,
    vector_index: i32,
}

impl IntermediateNode {
    fn new(tok: Token, is_float: i32) -> IntermediateNode {
        IntermediateNode {
            tok,
            is_float,
            history_index: 0,
            vector_index: 0,
        }
    }
}

#[derive(Clone, Debug)]
enum StackItem {
    State(StateT),
    Node(Vec<IntermediateNode>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StateT {
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
    ClosePAREN,
    Comma,
    End,
}

fn nodes_from_intermediate(nodes: &[IntermediateNode]) -> ExprNode {
    // Build linked list from the back.
    let mut next_arc: Option<Arc<ExprNode>> = None;
    for inode in nodes.iter().rev() {
        let n = ExprNode {
            tok: inode.tok,
            is_float: inode.is_float,
            history_index: inode.history_index,
            vector_index: inode.vector_index,
            next: next_arc.take(),
        };
        next_arc = Some(Arc::new(n));
    }
    // Unwrap the Arc into a value (build front node by hand if list non-empty).
    if let Some(arc) = next_arc {
        let head = &nodes[0];
        // Reconstruct a fresh head whose `next` points to the rest of the chain.
        // The first Arc we built actually contains the head already.
        // Decompose: clone the head fields and use the rest.
        ExprNode {
            tok: head.tok,
            is_float: head.is_float,
            history_index: head.history_index,
            vector_index: head.vector_index,
            next: arc.next.clone(),
        }
    } else {
        ExprNode::new()
    }
}

fn collapse_intermediate_to_left(
    lhs: &mut Vec<IntermediateNode>,
    rhs: Vec<IntermediateNode>,
    constant_folding: bool,
) {
    let mut refvar = false;
    for n in lhs.iter() {
        if n.tok.token_type == TokenType::Var {
            refvar = true;
        }
    }
    for n in rhs.iter() {
        if n.tok.token_type == TokenType::Var {
            refvar = true;
        }
    }

    // The "trailing operator" on the LHS in the original C is the last node
    // (an operator, since lhs ends with a pending op). The insertion point is
    // BEFORE that trailing operator. In the C code, `plhs_last` walks to the
    // last node and inserts before it. But actually, looking again: it walks
    // `*plhs_last` to the end, then inserts the rhs *after* `*plhs_last`'s
    // current position by splicing. Let me re-read:
    //
    // C code does:
    //   plhs_last = &(*plhs_last)->next;  // points past last node
    //   ... rhs_last->next = (*plhs_last);  // = NULL
    //   (*plhs_last) = rhs;  // append rhs at end
    //
    // Wait - look again carefully. plhs_last starts as plhs (pointer to head
    // pointer). Then while ((*plhs_last)->next) plhs_last = &(*plhs_last)->next.
    // So at end, plhs_last points to the (*plhs_last)->next field of the LAST
    // node. The LAST node is the trailing operator.
    //
    // Then coercion logic uses (*plhs_last) which is the last node (op).
    // Wait no - after the loop, *plhs_last is still pointing at the last node?
    // Let me re-read... Actually `plhs_last = &(*plhs_last)->next` advances
    // by one. The loop continues while (*plhs_last)->next is non-null. So at
    // exit, *plhs_last is the last node (whose ->next is NULL).
    //
    // So *plhs_last refers to the last node in lhs (the trailing operator).
    // The coercion check is on `(*plhs_last)->is_float`. Then:
    //   rhs_last->next = (*plhs_last);
    //   (*plhs_last) = rhs;
    // This says: at the position where *plhs_last currently is, splice in rhs
    // and have rhs_last's next point to what was at that position.
    // But plhs_last is &(prev->next), so this REPLACES the trailing op with
    // the rhs and appends the trailing op after rhs.
    //
    // Wait that's not right either. plhs_last starts at &plhs (so *plhs_last
    // is the head pointer). Loop: while (*plhs_last)->next != NULL, advance
    // plhs_last to &(*plhs_last)->next. So if there are nodes A -> B -> C
    // (C->next = NULL), plhs_last starts as &head (so *plhs_last = A which
    // has next != NULL). Advance: plhs_last = &A->next, *plhs_last = B (next
    // != NULL). Advance: plhs_last = &B->next, *plhs_last = C (next == NULL).
    // Stop. So *plhs_last is the LAST node (C).
    //
    // Then: rhs_last->next = (*plhs_last);  // rhs_last->next = C
    // And:  (*plhs_last) = rhs;             // B->next = rhs
    //
    // So the splice is: B -> rhs ... -> rhs_last -> C
    //
    // i.e. rhs is inserted BEFORE the trailing operator (last node) of lhs.
    //
    // OK so in our flat representation: take lhs.last() out (trailing op),
    // append rhs, then re-append the trailing op. With coercion based on
    // is_float of (the last node = trailing op) and rhs_last.

    if lhs.is_empty() {
        *lhs = rhs;
        return;
    }
    let trailing = lhs.pop().unwrap();
    let mut rhs = rhs;
    let trailing_is_float = trailing.is_float != 0;
    let rhs_last_is_float = rhs.last().map(|n| n.is_float != 0).unwrap_or(false);
    let is_float = trailing_is_float || rhs_last_is_float;

    let coerce = make_token(TokenType::ToFloat);
    if trailing_is_float && !rhs_last_is_float {
        // Append a float coercion to the end of rhs
        rhs.push(IntermediateNode::new(coerce, 1));
    } else if !trailing_is_float && rhs_last_is_float {
        // Insert a float coercion at the BEGINNING of trailing... but trailing
        // is just one node (the op). The C code does:
        //   exprnode e = exprnode_new(&coerce, 1);
        //   e->next = (*plhs_last);  // e->next = C (the trailing op)
        //   (*plhs_last) = e;        // B->next = e, so B -> e -> C
        //   plhs_last = &e->next;    // now plhs_last points to e->next
        //   e->next->is_float = 1;   // C->is_float = 1
        // After this, the splice puts rhs between e and C? Let me re-check.
        // After this re-assignment: *plhs_last is now e->next which is C.
        // Then later:
        //   rhs_last->next = (*plhs_last);  // rhs_last->next = C
        //   (*plhs_last) = rhs;             // e->next = rhs
        // So: B -> e -> rhs ... -> rhs_last -> C
        //
        // And C->is_float was set to 1.
        let mut ce = IntermediateNode::new(coerce, 1);
        ce.is_float = 1;
        // We need: lhs ... -> coerce -> rhs ... -> trailing(now is_float=1)
        lhs.push(ce);
        let mut new_trailing = trailing.clone();
        new_trailing.is_float = 1;
        lhs.append(&mut rhs);
        lhs.push(new_trailing);

        // Constant fold check uses is_float as combined.
        if constant_folding && !refvar {
            let folded = constant_fold(lhs.clone(), is_float);
            *lhs = folded;
        }
        return;
    }

    // Default path: append rhs then trailing
    lhs.append(&mut rhs);
    lhs.push(trailing);

    if constant_folding && !refvar {
        let folded = constant_fold(lhs.clone(), is_float);
        *lhs = folded;
    }
}

fn constant_fold(nodes: Vec<IntermediateNode>, is_float: bool) -> Vec<IntermediateNode> {
    // Build a temporary MapperExpr, evaluate it, replace with single literal.
    let head = nodes_from_intermediate(&nodes);
    let mut tmp = MapperExpr {
        node: head,
        vector_size: 1,
        history_size: 1,
        history_pos: -1,
        input_history: vec![MapperSignalValue::F(0.0)],
        output_history: vec![MapperSignalValue::F(0.0)],
    };
    let dummy_input = MapperSignalValue::F(0.0);
    let result = evaluate_internal(&mut tmp, None, &dummy_input);

    let mut tok = if is_float {
        let mut t = make_token(TokenType::Float);
        let f = match result {
            MapperSignalValue::F(f) => f,
            MapperSignalValue::I32(i) => i as f32,
        };
        t.value = Some(f);
        t
    } else {
        let mut t = make_token(TokenType::Int);
        let i = match result {
            MapperSignalValue::I32(i) => i,
            MapperSignalValue::F(f) => f as i32,
        };
        t.int_value = Some(i);
        t
    };
    let _ = &mut tok;
    let mut node = IntermediateNode::new(tok, if is_float { 1 } else { 0 });
    node.is_float = if is_float { 1 } else { 0 };
    vec![node]
}

fn append_op_to_top(top_nodes: &mut Vec<IntermediateNode>, tok: Token) {
    // The C macro APPEND_OP appends an op node with is_float = current trailing
    // is_float to the end of the top stack node.
    let trailing_is_float = top_nodes.last().map(|n| n.is_float).unwrap_or(0);
    let mut n = IntermediateNode::new(tok, 0);
    n.is_float = trailing_is_float;
    top_nodes.push(n);
}

pub fn mapper_expr_new_from_string(
    s: &str,
    input_is_float: i32,
    output_is_float: i32,
    vector_size: i32,
) -> MapperExpr {
    let make_empty = || MapperExpr {
        node: ExprNode::new(),
        vector_size: 0,
        history_size: 0,
        history_pos: -1,
        input_history: Vec::new(),
        output_history: Vec::new(),
    };
    if s.is_empty() {
        return make_empty();
    }
    let tokens = expr_lex(vec![s]);
    let mut tok_idx = 0usize;
    let mut next_token = true;
    let mut tok = tokens[0];

    let mut stack: Vec<StackItem> = Vec::new();
    let mut var_allowed = true;
    let mut oldest_samps: f32 = 0.0;
    let mut error: Option<&'static str> = None;
    let mut result: Option<Vec<IntermediateNode>> = None;

    stack.push(StackItem::State(StateT::Expr));
    stack.push(StackItem::State(StateT::YEqualEq));
    stack.push(StackItem::State(StateT::YEqualY));

    while !stack.is_empty() {
        if next_token {
            if tok_idx >= tokens.len() {
                error = Some("Lex error: out of tokens");
                break;
            }
            tok = tokens[tok_idx];
            tok_idx += 1;
            next_token = false;
        }

        let top_idx = stack.len() - 1;
        // If top of stack is a Node:
        let is_top_node = matches!(stack[top_idx], StackItem::Node(_));
        if is_top_node {
            if top_idx == 0 {
                // pop and return as result
                if let StackItem::Node(n) = stack.pop().unwrap() {
                    result = Some(n);
                }
                break;
            }
            let top_minus_1 = top_idx - 1;
            // top-1 must be a state (per logic in C)
            let prev_is_state = matches!(stack[top_minus_1], StackItem::State(_));
            if prev_is_state {
                if top_minus_1 >= 1 && matches!(stack[top_minus_1 - 1], StackItem::Node(_)) {
                    // We have node, state, node from top-2 to top.
                    let st = if let StackItem::State(s) = stack[top_minus_1] {
                        s
                    } else {
                        unreachable!()
                    };
                    match st {
                        StateT::ExprRight | StateT::TermRight | StateT::ClosePAREN => {
                            let top_node = if let StackItem::Node(n) = stack.pop().unwrap() {
                                n
                            } else {
                                unreachable!()
                            };
                            // pop the state (no, in C: collapse_expr_to_left then POP())
                            // Actually in C: collapse and POP() once. Look:
                            //   collapse_expr_to_left(&stack[top-2].node, stack[top].node, 1);
                            //   POP();
                            // So only one POP. The state remains on the stack. But we
                            // already popped the top node. That brings the state to top.
                            // The state stays. Let's re-examine: original stack indexing
                            // after popping = [..., node@top-2, state@top-1]. State stays.
                            // We need: lhs = stack[top-2] node, mutate it.
                            let lhs_idx = top_minus_1 - 1;
                            if let StackItem::Node(lhs) = &mut stack[lhs_idx] {
                                collapse_intermediate_to_left(lhs, top_node, true);
                            }
                        }
                        StateT::CloseHistIndex => {
                            // Pull off the index expression node, set history_index of var two-down
                            let top_node = if let StackItem::Node(n) = stack.pop().unwrap() {
                                n
                            } else {
                                unreachable!()
                            };
                            // The state stays on the stack. Get var node which is top-2 in old
                            // indexing = lhs_idx now since we popped one.
                            let lhs_idx = top_minus_1 - 1;
                            // top_node should be a single int/float
                            if top_node.len() != 1 {
                                error = Some("expected lonely INT or FLOAT in history index");
                                break;
                            }
                            let val = &top_node[0];
                            let idx_value: i32 = match val.tok.token_type {
                                TokenType::Float => val.tok.value.unwrap_or(0.0) as i32,
                                TokenType::Int => val.tok.int_value.unwrap_or(0),
                                _ => {
                                    error = Some("expected INT or FLOAT in history index");
                                    break;
                                }
                            };
                            if let StackItem::Node(var_node) = &mut stack[lhs_idx] {
                                if let Some(n) = var_node.first_mut() {
                                    if n.tok.token_type == TokenType::Var {
                                        n.history_index = idx_value;
                                        if (oldest_samps as i32) > n.history_index {
                                            oldest_samps = n.history_index as f32;
                                        }
                                    } else {
                                        error = Some("expected VAR two-down on the stack.");
                                        break;
                                    }
                                }
                            }
                        }
                        StateT::CloseVectIndex => {
                            let top_node = if let StackItem::Node(n) = stack.pop().unwrap() {
                                n
                            } else {
                                unreachable!()
                            };
                            let lhs_idx = top_minus_1 - 1;
                            if top_node.len() != 1 {
                                error = Some("expected lonely INT or FLOAT in vector index");
                                break;
                            }
                            let val = &top_node[0];
                            let idx_value: i32 = match val.tok.token_type {
                                TokenType::Float => val.tok.value.unwrap_or(0.0) as i32,
                                TokenType::Int => val.tok.int_value.unwrap_or(0),
                                _ => {
                                    error = Some("expected INT or FLOAT in vector index");
                                    break;
                                }
                            };
                            if let StackItem::Node(var_node) = &mut stack[lhs_idx] {
                                if let Some(n) = var_node.first_mut() {
                                    if n.tok.token_type == TokenType::Var {
                                        n.vector_index = idx_value;
                                        if n.vector_index > 0 {
                                            error = Some("Vector indexing not yet implemented.");
                                            break;
                                        }
                                        if n.vector_index < 0 || n.vector_index >= vector_size {
                                            error = Some("Vector index outside input size.");
                                            break;
                                        }
                                    } else {
                                        error = Some("expected VAR two-down on the stack.");
                                        break;
                                    }
                                }
                            }
                        }
                        _ => {
                            // swap node down: stack[top-1] <-> stack[top]
                            stack.swap(top_idx, top_minus_1);
                        }
                    }
                } else {
                    // swap the node with the state below it
                    stack.swap(top_idx, top_minus_1);
                }
            }
            continue;
        }

        // Top is a state. Process it.
        let st = if let StackItem::State(s) = stack[top_idx] {
            s
        } else {
            unreachable!()
        };

        match st {
            StateT::YEqualY => {
                if tok.token_type == TokenType::Var && tok.var == Some('y') {
                    stack.pop();
                } else {
                    error = Some("Error in y= prefix.");
                    break;
                }
                next_token = true;
            }
            StateT::YEqualEq => {
                if tok.token_type == TokenType::Op && tok.op == Some('=') {
                    stack.pop();
                } else {
                    error = Some("Error in y= prefix.");
                    break;
                }
                next_token = true;
            }
            StateT::Expr => {
                stack.pop();
                stack.push(StackItem::State(StateT::ExprRight));
                stack.push(StackItem::State(StateT::Term));
            }
            StateT::ExprRight => {
                if tok.token_type == TokenType::Op {
                    stack.pop();
                    if tok.op == Some('+') || tok.op == Some('-') {
                        // APPEND_OP to top node (which is now top after pop)
                        if let Some(StackItem::Node(n)) = stack.last_mut() {
                            append_op_to_top(n, tok);
                        }
                        stack.push(StackItem::State(StateT::Expr));
                        next_token = true;
                    }
                } else {
                    stack.pop();
                }
            }
            StateT::Term => {
                stack.pop();
                stack.push(StackItem::State(StateT::TermRight));
                stack.push(StackItem::State(StateT::Value));
            }
            StateT::TermRight => {
                if tok.token_type == TokenType::Op {
                    stack.pop();
                    if tok.op == Some('*') || tok.op == Some('/') {
                        if let Some(StackItem::Node(n)) = stack.last_mut() {
                            append_op_to_top(n, tok);
                        }
                        stack.push(StackItem::State(StateT::Term));
                        next_token = true;
                    }
                } else {
                    stack.pop();
                }
            }
            StateT::Value => {
                if tok.token_type == TokenType::Int {
                    stack.pop();
                    stack.push(StackItem::Node(vec![IntermediateNode::new(tok, 0)]));
                    next_token = true;
                } else if tok.token_type == TokenType::Float {
                    stack.pop();
                    stack.push(StackItem::Node(vec![IntermediateNode::new(tok, 1)]));
                    next_token = true;
                } else if tok.token_type == TokenType::Var {
                    if var_allowed {
                        stack.pop();
                        stack.push(StackItem::Node(vec![IntermediateNode::new(
                            tok,
                            input_is_float,
                        )]));
                        stack.push(StackItem::State(StateT::VarRight));
                        next_token = true;
                    } else {
                        error = Some("Unexpected variable reference.");
                        break;
                    }
                } else if tok.token_type == TokenType::OpenParen {
                    stack.pop();
                    stack.push(StackItem::State(StateT::ClosePAREN));
                    stack.push(StackItem::State(StateT::Expr));
                    next_token = true;
                } else if tok.token_type == TokenType::Func {
                    stack.pop();
                    let func_idx = tok.int_value.unwrap_or(-1);
                    if func_idx < 0 {
                        error = Some("Unknown function.");
                        break;
                    }
                    stack.push(StackItem::Node(vec![IntermediateNode::new(tok, 1)]));
                    let arity = FUNCTION_LIST[func_idx as usize].arity;
                    if arity > 0 {
                        stack.push(StackItem::State(StateT::ClosePAREN));
                        stack.push(StackItem::State(StateT::Expr));
                        for _ in 1..arity {
                            stack.push(StackItem::State(StateT::Comma));
                            stack.push(StackItem::State(StateT::Expr));
                        }
                        stack.push(StackItem::State(StateT::OpenParen));
                    }
                    next_token = true;
                } else if tok.token_type == TokenType::Op && tok.op == Some('-') {
                    stack.pop();
                    stack.push(StackItem::State(StateT::Negate));
                    stack.push(StackItem::State(StateT::Value));
                    next_token = true;
                } else {
                    error = Some("Expected value.");
                    break;
                }
            }
            StateT::Negate => {
                stack.pop();
                let new_top_idx = stack.len() - 1;
                if let StackItem::Node(rhs_nodes) = stack[new_top_idx].clone() {
                    let mut zero_tok = make_token(TokenType::Int);
                    zero_tok.int_value = Some(0);
                    let mut minus_tok = make_token(TokenType::Op);
                    minus_tok.op = Some('-');
                    let mut e: Vec<IntermediateNode> = Vec::new();
                    e.push(IntermediateNode::new(zero_tok, 0));
                    e.push(IntermediateNode::new(minus_tok, 0));
                    collapse_intermediate_to_left(&mut e, rhs_nodes, true);
                    stack[new_top_idx] = StackItem::Node(e);
                } else {
                    error = Some("Expected to negate an expression.");
                    break;
                }
            }
            StateT::VarRight => {
                if tok.token_type == TokenType::OpenSquare {
                    stack.pop();
                    stack.push(StackItem::State(StateT::VarVectIndex));
                } else if tok.token_type == TokenType::OpenCurly {
                    stack.pop();
                    stack.push(StackItem::State(StateT::VarHistIndex));
                } else {
                    stack.pop();
                }
            }
            StateT::VarVectIndex => {
                stack.pop();
                if tok.token_type == TokenType::OpenSquare {
                    var_allowed = false;
                    stack.push(StackItem::State(StateT::CloseVectIndex));
                    stack.push(StackItem::State(StateT::Expr));
                    next_token = true;
                }
            }
            StateT::VarHistIndex => {
                stack.pop();
                if tok.token_type == TokenType::OpenCurly {
                    var_allowed = false;
                    stack.push(StackItem::State(StateT::CloseHistIndex));
                    stack.push(StackItem::State(StateT::Expr));
                    next_token = true;
                }
            }
            StateT::CloseVectIndex => {
                if tok.token_type == TokenType::CloseSquare {
                    var_allowed = true;
                    stack.pop();
                    stack.push(StackItem::State(StateT::VarHistIndex));
                    next_token = true;
                } else {
                    error = Some("Expected ']'.");
                    break;
                }
            }
            StateT::CloseHistIndex => {
                if tok.token_type == TokenType::CloseCurly {
                    var_allowed = true;
                    stack.pop();
                    stack.push(StackItem::State(StateT::VarVectIndex));
                    next_token = true;
                } else {
                    error = Some("Expected '}'.");
                    break;
                }
            }
            StateT::ClosePAREN => {
                if tok.token_type == TokenType::CloseParen {
                    stack.pop();
                    next_token = true;
                } else {
                    error = Some("Expected ')'.");
                    break;
                }
            }
            StateT::Comma => {
                if tok.token_type == TokenType::Comma {
                    stack.pop();
                    // find previous expression on the stack and collapse top-of-stack onto it
                    let cur_top_idx = stack.len() - 1;
                    // top should be a node now
                    let top_node = if let StackItem::Node(_) = stack[cur_top_idx] {
                        if let StackItem::Node(n) = stack.pop().unwrap() {
                            Some(n)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    if let Some(top_node) = top_node {
                        // Search backwards for previous Node
                        let mut prev_idx: Option<usize> = None;
                        for j in (0..stack.len()).rev() {
                            if let StackItem::Node(_) = stack[j] {
                                prev_idx = Some(j);
                                break;
                            }
                        }
                        if let Some(j) = prev_idx {
                            if let StackItem::Node(lhs) = &mut stack[j] {
                                collapse_intermediate_to_left(lhs, top_node, false);
                            }
                        }
                    }
                    next_token = true;
                } else {
                    error = Some("Expected ','.");
                    break;
                }
            }
            StateT::OpenParen => {
                if tok.token_type == TokenType::OpenParen {
                    stack.pop();
                    next_token = true;
                } else {
                    error = Some("Expected '('.");
                    break;
                }
            }
            StateT::End => {
                if tok.token_type == TokenType::End {
                    stack.pop();
                } else {
                    error = Some("Expected END.");
                    break;
                }
            }
        }
    }

    if error.is_some() || result.is_none() {
        if let Some(e) = error {
            println!("{}", e);
        }
        return make_empty();
    }
    let mut nodes = result.unwrap();

    if oldest_samps < -100.0 {
        return make_empty();
    }

    // Coerce final output
    let last_is_float = nodes.last().map(|n| n.is_float != 0).unwrap_or(false);
    if last_is_float && output_is_float == 0 {
        let mut t = make_token(TokenType::ToInt32);
        let _ = &mut t;
        let mut node = IntermediateNode::new(t, 0);
        node.is_float = 0;
        nodes.push(node);
    } else if !last_is_float && output_is_float != 0 {
        let mut t = make_token(TokenType::ToFloat);
        let _ = &mut t;
        let mut node = IntermediateNode::new(t, 0);
        node.is_float = 1;
        nodes.push(node);
    }

    // Special case for vector_size > 1: forbid vector_index > 0
    if vector_size > 1 {
        for n in nodes.iter() {
            if n.tok.token_type == TokenType::Var && n.vector_index > 0 {
                return make_empty();
            }
        }
    }

    let head = nodes_from_intermediate(&nodes);
    let history_size = ((-oldest_samps).ceil() as i32) + 1;
    let total_in = (vector_size * history_size) as usize;
    let total_out = history_size as usize;

    MapperExpr {
        node: head,
        vector_size,
        history_size,
        history_pos: -1,
        input_history: vec![MapperSignalValue::F(0.0); total_in],
        output_history: vec![MapperSignalValue::F(0.0); total_out],
    }
}

fn evaluate_internal(
    expr: &mut MapperExpr,
    input_vector: Option<&MapperSignalValue>,
    _orig_input: &MapperSignalValue,
) -> MapperSignalValue {
    // Stack of mapper signal values (use union semantics: we track both f and i32)
    #[derive(Clone, Copy, Debug)]
    struct Cell {
        f: f32,
        i32_v: i32,
    }
    impl Cell {
        fn from_f(f: f32) -> Cell {
            Cell { f, i32_v: 0 }
        }
        fn from_i(i: i32) -> Cell {
            Cell { f: 0.0, i32_v: i }
        }
        fn from_msv(v: &MapperSignalValue) -> Cell {
            match v {
                MapperSignalValue::F(f) => Cell {
                    f: *f,
                    i32_v: f.to_bits() as i32,
                },
                MapperSignalValue::I32(i) => Cell {
                    f: f32::from_bits(*i as u32),
                    i32_v: *i,
                },
            }
        }
    }
    let mut stack: Vec<Cell> = Vec::with_capacity(STACK_SIZE);

    if input_vector.is_some() {
        expr.history_pos = (expr.history_pos + 1).rem_euclid(expr.history_size.max(1));
        if let Some(input) = input_vector {
            let pos = (expr.history_pos * expr.vector_size) as usize;
            // Treat input_vector as a single-element vector (since vector_size==1 for our cases)
            // copy expr.vector_size elements from input. We only have one MapperSignalValue here.
            for i in 0..expr.vector_size as usize {
                let src = if i == 0 { *input } else { *input };
                if pos + i < expr.input_history.len() {
                    expr.input_history[pos + i] = src;
                }
            }
        }
    }

    let mut node_arc: Option<Arc<ExprNode>> = Some(Arc::new(ExprNode {
        tok: expr.node.tok,
        is_float: expr.node.is_float,
        history_index: expr.node.history_index,
        vector_index: expr.node.vector_index,
        next: expr.node.next.clone(),
    }));

    let mut error_occurred = false;

    while let Some(node) = node_arc {
        let n = &*node;
        match n.tok.token_type {
            TokenType::Int => {
                stack.push(Cell::from_i(n.tok.int_value.unwrap_or(0)));
            }
            TokenType::Float => {
                stack.push(Cell::from_f(n.tok.value.unwrap_or(0.0)));
            }
            TokenType::Var => {
                let hp: i32 = expr.history_pos;
                let hs: i32 = expr.history_size.max(1);
                let idx: i32 = ((n.history_index + hp + hs) % hs).max(0);
                let var_char: char = n.tok.var.unwrap_or(' ');
                if var_char == 'x' {
                    let real_idx: usize =
                        (idx * expr.vector_size + n.vector_index) as usize;
                    if real_idx < expr.input_history.len() {
                        stack.push(Cell::from_msv(&expr.input_history[real_idx]));
                    } else {
                        error_occurred = true;
                        break;
                    }
                } else if var_char == 'y' {
                    let real_idx: usize = idx as usize;
                    if real_idx < expr.output_history.len() {
                        stack.push(Cell::from_msv(&expr.output_history[real_idx]));
                    } else {
                        error_occurred = true;
                        break;
                    }
                } else {
                    error_occurred = true;
                    break;
                }
            }
            TokenType::ToFloat => {
                if let Some(top) = stack.last_mut() {
                    top.f = top.i32_v as f32;
                }
            }
            TokenType::ToInt32 => {
                if let Some(top) = stack.last_mut() {
                    top.i32_v = top.f as i32;
                }
            }
            TokenType::Op => {
                if stack.len() < 2 {
                    error_occurred = true;
                    break;
                }
                let right = stack.pop().unwrap();
                let left = stack.pop().unwrap();
                let op = n.tok.op.unwrap_or('+');
                if n.is_float != 0 {
                    let r = match op {
                        '+' => left.f + right.f,
                        '-' => left.f - right.f,
                        '*' => left.f * right.f,
                        '/' => left.f / right.f,
                        _ => {
                            error_occurred = true;
                            break;
                        }
                    };
                    stack.push(Cell::from_f(r));
                } else {
                    let r = match op {
                        '+' => left.i32_v.wrapping_add(right.i32_v),
                        '-' => left.i32_v.wrapping_sub(right.i32_v),
                        '*' => left.i32_v.wrapping_mul(right.i32_v),
                        '/' => {
                            if right.i32_v == 0 {
                                0
                            } else {
                                left.i32_v.wrapping_div(right.i32_v)
                            }
                        }
                        _ => {
                            error_occurred = true;
                            break;
                        }
                    };
                    stack.push(Cell::from_i(r));
                }
            }
            TokenType::Func => {
                let fidx = n.tok.int_value.unwrap_or(-1);
                if fidx < 0 || (fidx as usize) >= FUNCTION_LIST.len() {
                    error_occurred = true;
                    break;
                }
                let entry = FUNCTION_LIST[fidx as usize];
                match entry.arity {
                    0 => {
                        let v = (entry.func)(0.0, 0.0);
                        stack.push(Cell::from_f(v));
                    }
                    1 => {
                        if stack.is_empty() {
                            error_occurred = true;
                            break;
                        }
                        let r = stack.pop().unwrap();
                        let v = (entry.func)(r.f, 0.0);
                        stack.push(Cell::from_f(v));
                    }
                    2 => {
                        if stack.len() < 2 {
                            error_occurred = true;
                            break;
                        }
                        let r = stack.pop().unwrap();
                        let l = stack.pop().unwrap();
                        let v = (entry.func)(l.f, r.f);
                        stack.push(Cell::from_f(v));
                    }
                    _ => {
                        error_occurred = true;
                        break;
                    }
                }
            }
            _ => {
                error_occurred = true;
                break;
            }
        }
        node_arc = n.next.clone();
    }

    if error_occurred || stack.is_empty() {
        return MapperSignalValue::I32(0);
    }
    let top = stack[0];
    // Determine which slot is meaningful by looking at last node's is_float
    // Find last node:
    let mut last_is_float = expr.node.is_float != 0;
    let mut cur = expr.node.next.clone();
    let mut last_tok_type = expr.node.tok.token_type;
    while let Some(c) = cur {
        last_is_float = c.is_float != 0;
        last_tok_type = c.tok.token_type;
        cur = c.next.clone();
    }
    let result = if last_tok_type == TokenType::ToInt32 {
        MapperSignalValue::I32(top.i32_v)
    } else if last_tok_type == TokenType::ToFloat {
        MapperSignalValue::F(top.f)
    } else if last_is_float {
        MapperSignalValue::F(top.f)
    } else {
        MapperSignalValue::I32(top.i32_v)
    };

    if input_vector.is_some() {
        let hp = expr.history_pos as usize;
        if hp < expr.output_history.len() {
            expr.output_history[hp] = result;
        }
    }

    result
}

pub fn mapper_expr_evaluate<'a>(
    mapper: &mut MapperExpr,
    input: &'a MapperSignalValue,
) -> MapperSignalValue {
    evaluate_internal(mapper, Some(input), input)
}