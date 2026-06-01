#include "lexer.h"
#include "parser.h"
#include "compiler.h"
#include "vm.h"
#include "misc.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void test_program(const char* prog, const char* label) {
    printf("=== %s ===\n", label);
    int instr_num = 0;
    Instruction * instr = compile(prog, &instr_num);
    printf("instr_count=%d\n", instr_num);
    for (int i = 0; i < instr_num; i++) {
        if (instr[i].operand) {
            printf("  [%d] %s (%s)\n", i, get_opcode_name(instr[i].opcode), instr[i].operand);
        } else {
            printf("  [%d] %s *\n", i, get_opcode_name(instr[i].opcode));
        }
    }
    Frame * frame = init_frame();
    eval(frame, instr, instr_num);
    printf("after eval: sp=%d var_count=%d\n", frame->sp, frame->var_count);
    for (int i = 0; i < frame->var_count; i++) {
        printf("  var[%d]: %s = %d\n", i, frame->var_names[i], frame->variables[i]);
    }
    for (int i = 0; i <= frame->sp; i++) {
        printf("  stack[%d] = %d\n", i, frame->stack[i]);
    }
    printf("\n");
}

int main() {
    test_program("let x = 5 + 3 - 2;", "single let");
    test_program("let x = 5; let y = x + 3;", "two lets");
    test_program("let x = 5; x = x + 1;", "let then assign");
    test_program("let x = 100;", "single int");
    test_program("dis 42;", "dis literal");
    return 0;
}
