#include "mujs.h"
#include <stdio.h>
#include <string.h>
static void report(js_State *J, const char *m) { printf("[r] %s\n", m); }
#define TRY(call) do { \
	if (js_try(J)) { printf("caught: %s\n", js_tostring(J, -1)); js_pop(J, 1); } \
	else { call; js_endtry(J); printf("no throw?!\n"); } \
} while (0)
int main(void)
{
	js_State *J = js_newstate(NULL, NULL, 0);
	setvbuf(stdout, NULL, _IONBF, 0);
	js_setreport(J, report);
	TRY(js_error(J, "plain"));
	TRY(js_error(J, "str=%s int=%d", "abc", 42));
	TRY(js_typeerror(J, "%s/%s/%s/%s/%s/%s", "a", "b", "c", "d", "e", "f"));
	TRY(js_rangeerror(J, "%d %d %d %d %d %d %d %d %d %d", 1,2,3,4,5,6,7,8,9,10));
	TRY(js_syntaxerror(J, "double=%g %f %.3f %e", 1.5, 2.25, 3.14159, 1e10));
	TRY(js_referenceerror(J, "mixed %s %d %g %s %d %g", "s1", 7, 0.5, "s2", -3, 1e-5));
	TRY(js_urierror(J, "%c%c%c %05d %-8s| %x %X %o %%", 'x', 'y', 'z', 42, "pad", 255, 255, 8));
	TRY(js_evalerror(J, "%s", "eval"));
	TRY(js_error(J, "long %s", "0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789"));
	TRY(js_error(J, "%p-ptr", (void*)0));
	TRY(js_error(J, "%d %s %d %s %d %s %d %s", 1, "a", 2, "b", 3, "c", 4, "d"));
	TRY(js_error(J, "%.17g %.17g", 1.0/3.0, 1e-300));
	js_newerror(J, "newerror"); printf("newerror: %s\n", js_tostring(J, -1)); js_pop(J, 1);
	js_newtypeerror(J, "newtype"); printf("newtype: %s\n", js_tostring(J, -1)); js_pop(J, 1);
	js_newrangeerror(J, "newrange"); js_pop(J, 1);
	js_newreferenceerror(J, "newref"); js_pop(J, 1);
	js_newsyntaxerror(J, "newsyn"); js_pop(J, 1);
	js_newevalerror(J, "neweval"); js_pop(J, 1);
	js_newurierror(J, "newuri"); js_pop(J, 1);
	js_gc(J, 1);
	js_freestate(J);
	printf("vdone\n");
	return 0;
}
