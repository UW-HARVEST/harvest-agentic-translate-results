#include "../src/common.h"
#include "../src/parser.h"
#include "../src/reducer.h"
#include "../hash-table/hash_table.h"
#include <stdio.h>
#include <string.h>

int main() {
    char *r = alpha_convert("foo");
    printf("alpha_convert(foo) = %s\n", r);
    char *r2 = alpha_convert("bar");
    printf("alpha_convert(bar) = %s\n", r2);
    
    AstNode *v = create_variable("x", "Nat");
    char *s = ast_to_string(v);
    printf("ast_to_string(VAR x:Nat) = '%s'\n", s);
    
    AstNode *v2 = create_variable("y", NULL);
    char *s2 = ast_to_string(v2);
    printf("ast_to_string(VAR y:NULL) = '%s'\n", s2);
    
    AstNode *lam = create_lambda("z", v2, "Bool");
    char *s3 = ast_to_string(lam);
    printf("ast_to_string(LAMBDA z:Bool.y) = '%s'\n", s3);
    
    AstNode *app = create_application(v, v2);
    char *s4 = ast_to_string(app);
    printf("ast_to_string(APP) = '%s'\n", s4);
    
    return 0;
}
