// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the “Software”),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
// 
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
// 
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#ifndef MD_MACROS_H
#define MD_MACROS_H

#ifndef OP
#  define OP add
#endif
#ifndef REPEAT
#  define REPEAT 5
#endif

#define STR1(x) #x
#define STR(x)  STR1(x)
#define CAT1(a,b) a##b
#define CAT(a,b)  CAT1(a,b)

/* ---------- Operation family prototypes (defined in md_core.c) ---------- */
extern int op_add(int a, int b);
extern int op_sub(int a, int b);
extern int op_mul(int a, int b);

/* Choose function by building the identifier: op_<OP> */
#define OP_FN(op) CAT(op_, op)

/* Step macros (selected via token-paste) */
#define STEP_add(acc, i) ((acc) += (i))
#define STEP_sub(acc, i) ((acc) -= (i))
#define STEP_mul(acc, i) ((acc) *= ((i) + 1))

#define STEP_OP_I(op, acc, i) CAT(STEP_, op)(acc, (i))
#define STEP_OP(op, acc, i)   STEP_OP_I(op, acc, (i))

/* Initial accumulator per op (token-paste) */
#define INIT_add 0
#define INIT_sub 0
#define INIT_mul 1
#define INIT_FOR(op) CAT(INIT_, op)

/* ---- REPEAT-based manual unrolling ---- */

#define REP0(op, acc)        /* nothing */
#define REP1(op, acc)        STEP_OP(op, acc, 0);
#define REP2(op, acc)        REP1(op, acc) STEP_OP(op, acc, 1);
#define REP3(op, acc)        REP2(op, acc) STEP_OP(op, acc, 2);
#define REP4(op, acc)        REP3(op, acc) STEP_OP(op, acc, 3);
#define REP5(op, acc)        REP4(op, acc) STEP_OP(op, acc, 4);
#define REP6(op, acc)        REP5(op, acc) STEP_OP(op, acc, 5);
#define REP7(op, acc)        REP6(op, acc) STEP_OP(op, acc, 6);

/* Choose the REPn that matches REPEAT. */
#define CHOOSE_REP_I(n)      CAT(REP, n)
#define CHOOSE_REP(n)        CHOOSE_REP_I(n)

/* Loop macro expands into a real for-statement (Stmt comes from macros) */
#define FOR_EACH(n, body) for (int i = 0; i < (n); ++i) body
#define DO_LOOP(op, acc, n) FOR_EACH((n), { STEP_OP(op, acc, i); })
#define RUN_LOOP(op, acc, n) CHOOSE_REP(n)(op, acc)

/* Macro-generated accumulator function: accum_<OP>(n) */
#define DISPATCH_REP(op, acc, n) do { \
  switch (n) { \
    case 0: REP0(op, acc);  break; \
    case 1: REP1(op, acc);  break; \
    case 2: REP2(op, acc);  break; \
    case 3: REP3(op, acc);  break; \
    case 4: REP4(op, acc);  break; \
    case 5: REP5(op, acc);  break; \
    case 6: REP6(op, acc);  break; \
    default: break; \
  } \
} while(0)

#define DEFINE_ACCUM(op) \
  static int CAT(accum_, op)(int n) { \
    int acc = INIT_FOR(op); \
    DISPATCH_REP(op, acc, n); \
    return acc; \
  }
#define ACCUM_FN(op) CAT(accum_, op)

/* Globals provided by md_core.c */
extern int (*G_OP)(int, int);
extern const char *G_OP_NAME;

/* Helpers provided by md_core.c */
int helper_call(int a, int b);
int helper_ptr(int a, int b);
int use_generated(int n);

#endif /* MD_MACROS_H */
