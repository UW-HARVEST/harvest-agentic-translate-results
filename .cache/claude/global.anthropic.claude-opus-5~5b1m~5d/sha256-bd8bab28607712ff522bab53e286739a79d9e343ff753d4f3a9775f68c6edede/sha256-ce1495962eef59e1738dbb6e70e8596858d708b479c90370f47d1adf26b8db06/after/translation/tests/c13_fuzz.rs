//! Grammar-based whole-program fuzzer.
//!
//! The other tests target specific functions and specific `ERRORS.md` /
//! `CONFIGS.md` rows. This one generates random *programs* from a JavaScript
//! grammar and compares the full outcome, which is the broad safety net that
//! catches divergences in the INTERACTION of features (parser + compiler +
//! interpreter + builtins + GC) that no per-function test reaches.
//!
//! All seeds are fixed, so every failure is exactly reproducible.
mod common;
use common::*;
use std::ffi::c_int;

struct Gen {
    rng: Rng,
    depth: u32,
}

impl Gen {
    fn new(seed: u64) -> Gen {
        Gen { rng: Rng::new(seed), depth: 0 }
    }

    fn ident(&mut self) -> &'static str {
        *self.rng.pick(&["a", "b", "c", "x", "y", "z", "o", "arr", "f", "i", "n", "s"])
    }

    fn literal(&mut self) -> String {
        match self.rng.below(22) {
            0 => "0".into(),
            1 => "1".into(),
            2 => "-1".into(),
            3 => "0.5".into(),
            4 => "NaN".into(),
            5 => "Infinity".into(),
            6 => "-Infinity".into(),
            7 => "2147483648".into(),
            8 => "4294967296".into(),
            9 => "9007199254740993".into(),
            10 => "''".into(),
            11 => "'a'".into(),
            12 => "'0'".into(),
            13 => "'abc'".into(),
            14 => "'\\u00e9\\u4f60'".into(),
            15 => "true".into(),
            16 => "false".into(),
            17 => "null".into(),
            18 => "undefined".into(),
            19 => "[]".into(),
            20 => "({})".into(),
            _ => format!("{}", self.rng.range_i64(-1000, 1000)),
        }
    }

    fn expr(&mut self) -> String {
        self.depth += 1;
        let r = if self.depth > 5 {
            self.literal()
        } else {
            match self.rng.below(26) {
                0..=4 => self.literal(),
                5 => self.ident().to_string(),
                6 => {
                    let op = *self.rng.pick(&[
                        "+", "-", "*", "/", "%", "<", ">", "<=", ">=", "==", "!=", "===", "!==",
                        "&", "|", "^", "<<", ">>", ">>>", "&&", "||", ",",
                    ]);
                    format!("({} {} {})", self.expr(), op, self.expr())
                }
                7 => {
                    let op = *self.rng.pick(&["-", "+", "!", "~", "typeof ", "void "]);
                    format!("({op}{})", self.expr())
                }
                8 => format!("({} ? {} : {})", self.expr(), self.expr(), self.expr()),
                9 => format!("[{}, {}, {}]", self.expr(), self.expr(), self.expr()),
                10 => format!("({{p:{}, q:{}}})", self.expr(), self.expr()),
                11 => {
                    let id = self.ident();
                    format!("({id} = {})", self.expr())
                }
                12 => format!("({}).toString()", self.expr()),
                13 => format!("String({})", self.expr()),
                14 => format!("Number({})", self.expr()),
                15 => format!("Boolean({})", self.expr()),
                16 => {
                    let m = *self.rng.pick(&[
                        "abs", "ceil", "floor", "round", "sqrt", "exp", "log", "sin", "cos",
                    ]);
                    format!("Math.{m}({})", self.expr())
                }
                17 => format!("[{}, {}].join('|')", self.expr(), self.expr()),
                18 => format!("JSON.stringify({})", self.expr()),
                19 => format!("(function(p){{ return p }})({})", self.expr()),
                20 => format!("({}).length", self.expr()),
                21 => format!("arr[{}]", self.expr()),
                22 => format!("o[{}]", self.expr()),
                23 => {
                    let m = *self.rng.pick(&["charAt", "charCodeAt", "slice", "substring", "indexOf"]);
                    format!("String({}).{m}({})", self.expr(), self.expr())
                }
                24 => {
                    let m = *self.rng.pick(&["slice", "concat", "indexOf", "join"]);
                    format!("[1,2,3].{m}({})", self.expr())
                }
                _ => format!("(typeof {})", self.expr()),
            }
        };
        self.depth -= 1;
        r
    }

    fn stmt(&mut self) -> String {
        self.depth += 1;
        let r = if self.depth > 4 {
            format!("{};", self.expr())
        } else {
            match self.rng.below(18) {
                0..=4 => format!("{};", self.expr()),
                5 => format!("var {} = {};", self.ident(), self.expr()),
                6 => format!("if ({}) {{ {} }} else {{ {} }}", self.expr(), self.stmt(), self.stmt()),
                7 => format!(
                    "for (var i = 0; i < 3; i++) {{ {} }}",
                    self.stmt()
                ),
                8 => format!("for (var k in {}) {{ {} }}", self.expr(), self.stmt()),
                9 => format!(
                    "{{ var g = 0; while (g < 3) {{ g++; {} }} }}",
                    self.stmt()
                ),
                10 => format!("try {{ {} }} catch (e) {{ {} }}", self.stmt(), self.stmt()),
                11 => format!(
                    "try {{ {} }} catch (e) {{ {} }} finally {{ {} }}",
                    self.stmt(),
                    self.stmt(),
                    self.stmt()
                ),
                12 => format!("throw {};", self.expr()),
                13 => format!(
                    "switch ({}) {{ case 1: {} break; case 'a': {} default: {} }}",
                    self.expr(),
                    self.stmt(),
                    self.stmt(),
                    self.stmt()
                ),
                14 => format!(
                    "function h{}() {{ {} return {}; }}",
                    self.rng.below(4),
                    self.stmt(),
                    self.expr()
                ),
                15 => format!("do {{ {} }} while (false);", self.stmt()),
                16 => format!("with ({{p:1}}) {{ {} }}", self.stmt()),
                _ => format!("{} = {};", self.ident(), self.expr()),
            }
        };
        self.depth -= 1;
        r
    }

    /// A complete program: a prelude that defines the identifiers the generator
    /// uses (so most programs run rather than dying on the first ReferenceError),
    /// then random statements, then an expression whose value is compared.
    fn program(&mut self) -> String {
        let mut s = String::from(
            "var a=1,b=2,c=3,x=4,y=5,z=6,i=0,n=7,s='str';var o={p:1,q:'r'};var arr=[10,20,30];\
             var f=function(v){return v};var out=[];",
        );
        let n = 1 + self.rng.below(5);
        for _ in 0..n {
            s.push_str(&self.stmt());
        }
        // A final observable value that touches everything that might have changed.
        s.push_str(
            "String([typeof a,typeof b,typeof c,typeof x,typeof y,typeof z,String(a),String(b),\
             String(c),String(i),String(n),String(s),arr.join('|'),\
             (function(){var k=[];for(var p in o)k.push(p+'='+String(o[p]));return k.sort().join(',')})()\
             ].join(';'))",
        );
        s
    }
}

/// Generated programs can contain unbounded loops (e.g. a `while` whose body
/// reassigns the loop variable), so every fuzz state gets the SAME `js_setlimit`
/// runlimit. Both implementations then either finish or raise the literal
/// "script ran too long" -- and WHICH happens, at exactly which instruction
/// count, is itself part of what gets compared.
const FUZZ_RUNLIMIT: c_int = 200_000;

fn eval_bounded(imp: &Impl, flags: c_int, src: &[u8]) -> EvalOutcome {
    let j = imp.newstate(flags);
    imp.mute_report(j);
    imp.setlimit(j, FUZZ_RUNLIMIT, 0);
    let out = imp.eval_on(j, src);
    imp.freestate(j);
    out
}

fn check_bounded(b: &mut Batch, label: &str, flags: c_int, src: &[u8]) {
    let (c, r) = Impl::both();
    let a = eval_bounded(&c, flags, src);
    let bb = eval_bounded(&r, flags, src);
    b.checked += 1;
    if a != bb && b.failures.len() < 40 {
        b.failures.push(format!(
            "  {label} flags={flags} src={:?}\n      C   : {}\n      Rust: {}",
            show(src),
            a.pretty(),
            bb.pretty()
        ));
    }
}

fn fuzz(name: &str, seed: u64, iters: usize) {
    let mut g = Gen::new(seed);
    let mut b = Batch::new();
    for _ in 0..iters {
        let src = g.program();
        check_bounded(&mut b, "program", 0, src.as_bytes());
        check_bounded(&mut b, "program", JS_STRICT, src.as_bytes());
    }
    b.finish(name);
}

#[test]
fn fuzz_programs_seed_1() {
    fuzz("fuzz programs seed 1", 0xF022_0001, 1200);
}

#[test]
fn fuzz_programs_seed_2() {
    fuzz("fuzz programs seed 2", 0xF022_0002, 1200);
}

#[test]
fn fuzz_programs_seed_3() {
    fuzz("fuzz programs seed 3", 0xF022_0003, 1200);
}

#[test]
fn fuzz_expressions_only() {
    // Deeper expressions, no statements: concentrates on the operator and
    // coercion lattice.
    let mut g = Gen::new(0xF022_1000);
    let mut b = Batch::new();
    for _ in 0..4000 {
        g.depth = 0;
        let e = g.expr();
        let src = format!(
            "var a=1,b=2,c=3,x=4,y=5,z=6,i=0,n=7,s='str';var o={{p:1}};var arr=[10,20,30];\
             var f=function(v){{return v}}; String({e})"
        );
        check_bounded(&mut b, "expr", 0, src.as_bytes());
        check_bounded(&mut b, "expr", JS_STRICT, src.as_bytes());
    }
    b.finish("fuzz expressions");
}

#[test]
fn fuzz_source_text_is_lexed_and_parsed_identically() {
    // Feed the fuzzer's raw text through the compiler only (no execution), so
    // parse/compile errors are compared as well as successful runs. Also mutate
    // the text (delete / duplicate / replace random bytes) to reach the error
    // paths much more often.
    let mut g = Gen::new(0xF022_2000);
    let mut rng = Rng::new(0xF022_2001);
    let mut b = Batch::new();
    for _ in 0..4000 {
        let mut src = g.program().into_bytes();
        // mutate
        for _ in 0..rng.below(4) {
            if src.is_empty() {
                break;
            }
            let i = rng.below(src.len() as u32) as usize;
            match rng.below(3) {
                0 => {
                    src.remove(i);
                }
                1 => {
                    let ch = src[i];
                    src.insert(i, ch);
                }
                _ => {
                    src[i] = *rng.pick(b"(){}[]'\"/\\;,.:?!+-*%&|^~<>=@# \n\t0aA");
                }
            }
        }
        check_bounded(&mut b, "mutated", 0, &src);
        check_bounded(&mut b, "mutated", JS_STRICT, &src);
    }
    b.finish("fuzz mutated source text");
}

#[test]
fn fuzz_random_bytes_as_source() {
    // Pure random bytes: almost always a lexer error, and both impls must produce
    // the SAME error message (including for malformed UTF-8).
    let mut rng = Rng::new(0xF022_3000);
    let mut b = Batch::new();
    for _ in 0..12000 {
        let n = rng.below(40) as usize;
        let src: Vec<u8> = (0..n).map(|_| rng.next_u32() as u8).collect();
        for flags in [0 as c_int, JS_STRICT] {
            check_bounded(&mut b, "random bytes", flags, &src);
        }
    }
    b.finish("fuzz random bytes as source");
}

#[test]
fn fuzz_ascii_soup_as_source() {
    // Random printable ASCII: reaches deeper into the parser than random bytes.
    let mut rng = Rng::new(0xF022_4000);
    let mut b = Batch::new();
    for _ in 0..12000 {
        let n = rng.below(48) as usize;
        let src: Vec<u8> = (0..n)
            .map(|_| *rng.pick(b"abcxyz019(){}[]'\";,.:?!+-*/%&|^~<>= \n\t\\_$"))
            .collect();
        for flags in [0 as c_int, JS_STRICT] {
            check_bounded(&mut b, "ascii soup", flags, &src);
        }
    }
    b.finish("fuzz ascii soup as source");
}

#[test]
fn fuzz_shared_state_across_many_scripts() {
    // Run many generated scripts on the SAME state, so accumulated heap, the
    // interned-string table and the GC all participate -- a shape none of the
    // single-script tests reaches.
    let (c, r) = Impl::both();
    let mut b = Batch::new();
    for flags in [0 as c_int, JS_STRICT] {
        let jc = c.newstate(flags);
        let jr = r.newstate(flags);
        c.mute_report(jc);
        r.mute_report(jr);
        c.setlimit(jc, FUZZ_RUNLIMIT, 0);
        r.setlimit(jr, FUZZ_RUNLIMIT, 0);
        let mut g = Gen::new(0xF022_5000 ^ flags as u64);
        for k in 0..900 {
            let src = g.program();
            let a = c.eval_on(jc, src.as_bytes());
            let bb = r.eval_on(jr, src.as_bytes());
            b.checked += 1;
            if a != bb && b.failures.len() < 20 {
                b.failures.push(format!(
                    "  iteration {k} flags={flags} src={src:?}\n      C   : {}\n      Rust: {}",
                    a.pretty(),
                    bb.pretty()
                ));
            }
            if k % 100 == 0 {
                c.gc(jc, 0);
                r.gc(jr, 0);
            }
        }
        c.freestate(jc);
        r.freestate(jr);
    }
    b.finish("fuzz shared state across scripts");
}
