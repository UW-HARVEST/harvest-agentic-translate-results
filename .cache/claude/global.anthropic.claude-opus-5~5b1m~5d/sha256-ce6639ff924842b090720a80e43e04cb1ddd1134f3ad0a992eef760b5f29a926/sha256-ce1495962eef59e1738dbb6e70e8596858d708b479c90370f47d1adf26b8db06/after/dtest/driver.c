/* Differential test driver: linked against either the C or the Rust libmujs.so */
#include "mujs.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* internal, not in mujs.h */
extern void jsS_dumpstrings(js_State *J);
extern void js_insert(js_State *J, int idx);
extern int js_utflen(const char *s);
extern int js_utfptrtoidx(const char *s, const char *p);
extern int js_runeat(js_State *J, const char *s, int i);
extern double js_strtod(const char *as, char **aas);
extern double js_strtol(const char *s, char **ep, int radix);
extern int js_grisu2(double v, char *buffer, int *K);
extern void js_fmtexp(char *p, int e);
extern const char *js_itoa(char *buf, int a);
extern int jsU_chartorune(int *rune, const char *str);
extern int jsU_runetochar(char *str, const int *rune);
extern int jsU_runelen(int c);
extern int jsU_isalpharune(int c);
extern int jsU_islowerrune(int c);
extern int jsU_isupperrune(int c);
extern int jsU_tolowerrune(int c);
extern int jsU_toupperrune(int c);
extern int jsY_iswhite(int c);
extern int jsY_isnewline(int c);
extern int jsY_ishex(int c);
extern int jsY_tohex(int c);
extern const char *jsY_tokenstring(int token);
extern int jsV_numbertointeger(double n);
extern int jsV_numbertoint32(double n);
extern unsigned int jsV_numbertouint32(double n);
extern short jsV_numbertoint16(double n);
extern unsigned short jsV_numbertouint16(double n);
extern const char *jsV_numbertostring(js_State *J, char buf[32], double number);
extern double jsV_stringtonumber(js_State *J, const char *string);
extern double js_stringtofloat(const char *s, char **ep);
extern const char *js_intern(js_State *J, const char *s);
typedef struct Reprog Reprog;
typedef struct Resub { int nsub; struct { const char *sp; const char *ep; } sub[16]; } Resub;
extern Reprog *js_regcomp(const char *pattern, int cflags, const char **errorp);
extern int js_regexec(Reprog *prog, const char *string, Resub *sub, int eflags);
extern void js_regfree(Reprog *prog);


static void jsB_print(js_State *J)
{
	int i, top = js_gettop(J);
	for (i = 1; i < top; ++i) {
		const char *s = js_tostring(J, i);
		if (i > 1) putchar(' ');
		fputs(s, stdout);
	}
	putchar('\n');
	js_pushundefined(J);
}

static void jsB_repr(js_State *J)
{
	fputs(js_torepr(J, 1), stdout);
	putchar('\n');
	js_pushundefined(J);
}

static void jsB_gc(js_State *J)
{
	js_gc(J, 1);
	js_pushundefined(J);
}

static void myreport(js_State *J, const char *message)
{
	printf("[report] %s\n", message);
}

static void mypanic(js_State *J)
{
	printf("[panic]\n");
}

/* ---- userdata callbacks ---- */
static void ud_finalize(js_State *J, void *p) { printf("[finalize %s]\n", (char*)p); }
static int ud_has(js_State *J, void *p, const char *name)
{
	if (!strcmp(name, "magic")) { js_pushnumber(J, 42); return 1; }
	return 0;
}
static int ud_put(js_State *J, void *p, const char *name)
{
	printf("[put %s]\n", name);
	return !strcmp(name, "magic");
}
static int ud_delete(js_State *J, void *p, const char *name)
{
	printf("[delete %s]\n", name);
	return !strcmp(name, "magic");
}

static char *readfile(const char *fn)
{
	FILE *f = fopen(fn, "rb");
	long n;
	char *s;
	if (!f) { fprintf(stderr, "cannot open %s\n", fn); exit(1); }
	fseek(f, 0, SEEK_END);
	n = ftell(f);
	fseek(f, 0, SEEK_SET);
	s = malloc(n + 1);
	fread(s, 1, n, f);
	s[n] = 0;
	fclose(f);
	return s;
}

static void setup(js_State *J)
{
	js_setreport(J, myreport);
	js_atpanic(J, mypanic);
	js_newcfunction(J, jsB_print, "print", 1);
	js_setglobal(J, "print");
	js_newcfunction(J, jsB_repr, "repr", 1);
	js_setglobal(J, "repr");
	js_newcfunction(J, jsB_gc, "gc", 0);
	js_setglobal(J, "gc");
}

/* Exercise the C API surface itself. */
static void apitest(js_State *J)
{
	const char *ref;
	int okay;

	printf("-- api --\n");

	/* stack pushes and type predicates */
	js_pushundefined(J);
	js_pushnull(J);
	js_pushboolean(J, 7);
	js_pushnumber(J, 3.5);
	js_pushstring(J, "hello");
	js_pushlstring(J, "abcdef", 3);
	js_pushliteral(J, "literal");
	printf("gettop=%d\n", js_gettop(J));
	{
		int i;
		for (i = 0; i < js_gettop(J); ++i) {
			printf("%d: type=%d typeof=%s defined=%d null=%d bool=%d num=%d str=%d prim=%d obj=%d coercible=%d callable=%d\n",
				i, js_type(J, i), js_typeof(J, i), js_isdefined(J, i), js_isnull(J, i),
				js_isboolean(J, i), js_isnumber(J, i), js_isstring(J, i), js_isprimitive(J, i),
				js_isobject(J, i), js_iscoercible(J, i), js_iscallable(J, i));
			printf("   tostring='%s' tonumber=%.17g toboolean=%d tointeger=%d toint32=%d touint32=%u toint16=%d touint16=%u\n",
				js_tostring(J, i), js_tonumber(J, i), js_toboolean(J, i), js_tointeger(J, i),
				js_toint32(J, i), js_touint32(J, i), js_toint16(J, i), js_touint16(J, i));
			printf("   repr='%s'\n", js_torepr(J, i));
		}
	}
	js_pop(J, js_gettop(J));

	/* stack shuffling */
	js_pushnumber(J, 1); js_pushnumber(J, 2); js_pushnumber(J, 3); js_pushnumber(J, 4);
	js_rot4(J); printf("rot4: %g %g %g %g\n", js_tonumber(J,0), js_tonumber(J,1), js_tonumber(J,2), js_tonumber(J,3));
	js_rot3(J); printf("rot3: %g %g %g %g\n", js_tonumber(J,0), js_tonumber(J,1), js_tonumber(J,2), js_tonumber(J,3));
	js_rot2(J); printf("rot2: %g %g %g %g\n", js_tonumber(J,0), js_tonumber(J,1), js_tonumber(J,2), js_tonumber(J,3));
	js_rot(J, 4); printf("rot: %g %g %g %g\n", js_tonumber(J,0), js_tonumber(J,1), js_tonumber(J,2), js_tonumber(J,3));
	js_dup(J); js_dup2(J);
	printf("after dup: top=%d last=%g\n", js_gettop(J), js_tonumber(J, -1));
	js_rot2pop1(J); js_rot3pop2(J);
	printf("after rotpop: top=%d last=%g\n", js_gettop(J), js_tonumber(J, -1));
	js_copy(J, 0); printf("copy0=%g\n", js_tonumber(J, -1));
	js_remove(J, 0); printf("after remove top=%d\n", js_gettop(J));
	js_replace(J, 0); printf("after replace top=%d v0=%g\n", js_gettop(J), js_tonumber(J, 0));
	js_pop(J, js_gettop(J));

	/* objects and properties */
	js_newobject(J);
	js_pushnumber(J, 11); js_setproperty(J, -2, "a");
	js_pushstring(J, "bee"); js_defproperty(J, -2, "b", JS_DONTENUM);
	js_pushnumber(J, 3); js_setproperty(J, -2, "c");
	printf("hasa=%d ", js_hasproperty(J, -1, "a")); js_pop(J, 1);
	printf("hasz=%d\n", js_hasproperty(J, -1, "z"));
	js_getproperty(J, -1, "a"); printf("a=%g\n", js_tonumber(J, -1)); js_pop(J, 1);
	js_delproperty(J, -1, "c");
	js_getproperty(J, -1, "c"); printf("c after del: %s\n", js_tostring(J, -1)); js_pop(J, 1);
	js_setglobal(J, "obj");

	/* iterators */
	js_getglobal(J, "obj");
	js_pushiterator(J, -1, 1);
	while ((ref = js_nextiterator(J, -1)) != NULL)
		printf("iter own: %s\n", ref);
	js_pop(J, 2);
	js_getglobal(J, "obj");
	js_pushiterator(J, -1, 0);
	while ((ref = js_nextiterator(J, -1)) != NULL)
		printf("iter all: %s\n", ref);
	js_pop(J, 2);

	/* arrays and lengths */
	js_newarray(J);
	js_pushnumber(J, 100); js_setindex(J, -2, 0);
	js_pushnumber(J, 200); js_setindex(J, -2, 1);
	js_pushstring(J, "ss"); js_setindex(J, -2, 2);
	printf("array length=%d isarray=%d\n", js_getlength(J, -1), js_isarray(J, -1));
	printf("hasindex1=%d ", js_hasindex(J, -1, 1)); js_pop(J, 1);
	js_getindex(J, -1, 2); printf("idx2=%s\n", js_tostring(J, -1)); js_pop(J, 1);
	js_delindex(J, -1, 1);
	js_getindex(J, -1, 1); printf("idx1 after del=%s\n", js_tostring(J, -1)); js_pop(J, 1);
	js_setlength(J, -1, 8);
	printf("array length now=%d repr=%s\n", js_getlength(J, -1), js_torepr(J, -1));
	js_setglobal(J, "arr");

	/* registry / refs */
	js_newobject(J);
	ref = js_ref(J);
	printf("ref=%s\n", ref[0] == '_' ? ref : "<addr>");
	js_getregistry(J, ref);
	printf("registry object=%d\n", js_isobject(J, -1));
	js_pop(J, 1);
	js_unref(J, ref);
	js_pushstring(J, "regval");
	js_setregistry(J, "myreg");
	js_getregistry(J, "myreg");
	printf("myreg=%s\n", js_tostring(J, -1));
	js_pop(J, 1);
	js_delregistry(J, "myreg");
	js_getregistry(J, "myreg");
	printf("myreg after del=%s\n", js_tostring(J, -1));
	js_pop(J, 1);

	/* globals */
	js_pushnumber(J, 5);
	js_defglobal(J, "gvar", JS_READONLY);
	js_getglobal(J, "gvar");
	printf("gvar=%g\n", js_tonumber(J, -1));
	js_pop(J, 1);
	js_delglobal(J, "gvar");

	/* userdata */
	js_newobject(J);
	js_newuserdatax(J, "MyTag", (void*)"udata", ud_has, ud_put, ud_delete, ud_finalize);
	printf("isuserdata=%d wrongtag=%d\n", js_isuserdata(J, -1, "MyTag"), js_isuserdata(J, -1, "Other"));
	printf("touserdata=%s\n", (char*)js_touserdata(J, -1, "MyTag"));
	js_getproperty(J, -1, "magic");
	printf("ud magic=%g\n", js_tonumber(J, -1));
	js_pop(J, 1);
	js_pushnumber(J, 1);
	js_setproperty(J, -2, "magic");
	js_delproperty(J, -1, "magic");
	js_pop(J, 1);

	/* accessors */
	js_newobject(J);
	js_dostring(J, "var getcount = 0;");
	js_getglobal(J, "Object");
	js_pop(J, 1);
	js_newcfunction(J, jsB_print, "getter", 0);
	js_pushnull(J);
	js_defaccessor(J, -3, "acc", JS_DONTENUM);
	js_setglobal(J, "accobj");

	/* function calls */
	js_dostring(J, "function add(a,b) { return a+b; }");
	js_getglobal(J, "add");
	js_pushnull(J);
	js_pushnumber(J, 40);
	js_pushnumber(J, 2);
	js_call(J, 2);
	printf("add(40,2)=%g\n", js_tonumber(J, -1));
	js_pop(J, 1);

	js_getglobal(J, "add");
	js_pushnull(J);
	js_pushstring(J, "x");
	js_pushstring(J, "y");
	if (js_pcall(J, 2)) printf("pcall error: %s\n", js_tostring(J, -1));
	else printf("pcall ok: %s\n", js_tostring(J, -1));
	js_pop(J, 1);

	js_dostring(J, "function Thing(v) { this.v = v; }");
	js_getglobal(J, "Thing");
	js_pushnumber(J, 9);
	if (js_pconstruct(J, 1)) printf("pconstruct error: %s\n", js_tostring(J, -1));
	else { js_getproperty(J, -1, "v"); printf("pconstruct v=%g\n", js_tonumber(J, -1)); js_pop(J, 1); }
	js_pop(J, 1);

	/* nonexistent function -> error path */
	js_pushnumber(J, 1);
	js_pushnull(J);
	if (js_pcall(J, 0)) printf("pcall on number: %s\n", js_tostring(J, -1));
	js_pop(J, 1);

	/* try* helpers */
	js_dostring(J, "var thrower = { toString: function() { throw new Error('boom'); }, valueOf: function() { throw new Error('boom2'); } };");
	js_getglobal(J, "thrower");
	printf("trystring=%s\n", js_trystring(J, -1, "DEFAULT"));
	printf("trynumber=%g\n", js_trynumber(J, -1, -1));
	printf("tryinteger=%d\n", js_tryinteger(J, -1, -1));
	printf("tryboolean=%d\n", js_tryboolean(J, -1, 0));
	printf("tryrepr=%s\n", js_tryrepr(J, -1, "DEFREPR"));
	js_pop(J, 1);

	/* comparisons */
	js_pushnumber(J, 1); js_pushstring(J, "1");
	printf("equal=%d strictequal=%d\n", js_equal(J), js_strictequal(J));
	printf("compare=%d okay=%d\n", js_compare(J, &okay), okay);
	js_pop(J, 2);
	js_pushstring(J, "a"); js_pushstring(J, "b");
	printf("compare2=%d\n", js_compare(J, &okay));
	js_concat(J);
	printf("concat=%s\n", js_tostring(J, -1));
	js_pop(J, 1);

	js_dostring(J, "var d = new Date(0); var n = new Number(1); var s = new String('x'); var b = new Boolean(1); var e = new Error('e'); var re = /x/g;");
	js_getglobal(J, "d"); printf("isdateobject=%d\n", js_isdateobject(J, -1)); js_pop(J, 1);
	js_getglobal(J, "n"); printf("isnumberobject=%d\n", js_isnumberobject(J, -1)); js_pop(J, 1);
	js_getglobal(J, "s"); printf("isstringobject=%d\n", js_isstringobject(J, -1)); js_pop(J, 1);
	js_getglobal(J, "b"); printf("isbooleanobject=%d\n", js_isbooleanobject(J, -1)); js_pop(J, 1);
	js_getglobal(J, "e"); printf("iserror=%d\n", js_iserror(J, -1)); js_pop(J, 1);
	js_getglobal(J, "re"); printf("isregexp=%d\n", js_isregexp(J, -1)); js_pop(J, 1);

	js_getglobal(J, "obj");
	js_getglobal(J, "Object");
	printf("instanceof=%d\n", js_instanceof(J));
	js_pop(J, 2);

	js_newregexp(J, "a+b", JS_REGEXP_G | JS_REGEXP_I);
	printf("newregexp source=%s\n", js_torepr(J, -1));
	js_setglobal(J, "re2");
	js_dostring(J, "print(re2.exec('xxAAB'), re2.lastIndex, re2.global, re2.ignoreCase, re2.multiline);");

	/* eval */
	js_pushstring(J, "1+2*3");
	js_eval(J);
	printf("eval=%g\n", js_tonumber(J, -1));
	js_pop(J, 1);

	/* ploadstring errors */
	if (js_ploadstring(J, "[bad]", "function ( {"))
		printf("ploadstring error: %s\n", js_tostring(J, -1));
	js_pop(J, 1);

	/* dostring error path */
	printf("dostring bad=%d\n", js_dostring(J, "throw new TypeError('from js')"));
	printf("dostring syntax=%d\n", js_dostring(J, "1 +* 2"));

	printf("-- api done --\n");
}


/* Exercise the internal (non-mujs.h) exported symbols directly. */
static void lowleveltest(js_State *J)
{
	char buf[64];
	char *end;
	int i, k, rune;
	const char *strs[] = { "", "a", "abc", "\xc3\xa9", "\xe4\xb8\xad\xe6\x96\x87", "a\xc3\xa9z", "\xf0\x9f\x98\x80" };
	double nums[] = { 0, -0.0, 1, -1, 0.5, 1e21, 1e-7, 1.0/3.0, 123456789, 1e300, 5e-324, 1e100 };
	const char *parses[] = { "0", "1", "  1.5", "1e10", "1e-10", "0x10", "-3", "+4", "abc", "1.7976931348623157e309",
		"9007199254740993", ".5", "5.", "1e", "e1", "Infinity", "-Infinity", "0.0000000001", "1e-400", "" };

	printf("-- lowlevel --\n");
	for (i = 0; i < (int)(sizeof strs / sizeof *strs); ++i) {
		printf("utflen(%s)=%d ptrtoidx=%d\n", strs[i], js_utflen(strs[i]), js_utfptrtoidx(strs[i], strs[i] + strlen(strs[i])));
		for (k = 0; k < js_utflen(strs[i]); ++k)
			printf("  runeat %d = %d\n", k, js_runeat(J, strs[i], k));
		k = jsU_chartorune(&rune, strs[i]);
		printf("  chartorune -> %d %d runelen=%d\n", k, rune, jsU_runelen(rune));
		k = jsU_runetochar(buf, &rune);
		buf[k] = 0;
		printf("  roundtrip %d '%s'\n", k, buf);
	}
	for (i = 0; i < 0x300; ++i)
		if (jsU_isalpharune(i) || jsU_islowerrune(i) || jsU_isupperrune(i))
			printf("rune %d: a=%d l=%d u=%d lower=%d upper=%d\n", i, jsU_isalpharune(i), jsU_islowerrune(i),
				jsU_isupperrune(i), jsU_tolowerrune(i), jsU_toupperrune(i));
	for (i = 0; i < (int)(sizeof nums / sizeof *nums); ++i) {
		int K = 0;
		int n = js_grisu2(nums[i], buf, &K);
		printf("grisu2(%.17g) = %d K=%d digits=", nums[i], n, K);
		for (k = 0; k < n; ++k) putchar(buf[k]);
		putchar('\n');
		printf("  numbertostring=%s int=%d i32=%d u32=%u i16=%d u16=%u\n",
			jsV_numbertostring(J, buf, nums[i]), jsV_numbertointeger(nums[i]), jsV_numbertoint32(nums[i]),
			jsV_numbertouint32(nums[i]), jsV_numbertoint16(nums[i]), jsV_numbertouint16(nums[i]));
	}
	for (i = -350; i <= 350; i += 7) {
		js_fmtexp(buf, i);
		printf("fmtexp(%d)=%s\n", i, buf);
	}
	for (i = 0; i < (int)(sizeof parses / sizeof *parses); ++i) {
		double d = js_strtod(parses[i], &end);
		printf("strtod('%s')=%.17g rest='%s'\n", parses[i], d, end);
		d = js_strtol(parses[i], &end, 10);
		printf("  strtol10=%.17g rest='%s'\n", d, end);
		d = js_strtol(parses[i], &end, 16);
		printf("  strtol16=%.17g rest='%s'\n", d, end);
		d = js_stringtofloat(parses[i], &end);
		printf("  stringtofloat=%.17g rest='%s'\n", d, end);
		printf("  stringtonumber=%.17g\n", jsV_stringtonumber(J, parses[i]));
	}
	for (i = -3; i < 40; ++i) {
		int v = (i == 0) ? -2147483647 - 1 : (i == 1) ? 2147483647 : i * 12345;
		printf("itoa(%d)=%s\n", v, js_itoa(buf, v));
	}
	for (i = 0; i < 300; ++i)
		if (jsY_iswhite(i) || jsY_isnewline(i) || jsY_ishex(i))
			printf("char %d: white=%d nl=%d hex=%d tohex=%d\n", i, jsY_iswhite(i), jsY_isnewline(i), jsY_ishex(i), jsY_tohex(i));
	for (i = 0; i < 320; ++i) {
		const char *ts = jsY_tokenstring(i);
		if (ts) printf("token %d = %s\n", i, ts);
	}
	printf("intern: %s %s\n", js_intern(J, "interned1"), js_intern(J, "interned2"));
	printf("-- lowlevel done --\n");
}


static void regexptest(void)
{
	static const char *pats[] = { "a", "(a)(b)?", "^x", "x$", "[[:alpha:]]", "a|b", "(a)\\1",
		"a{1,2}b", ".", "\\s+", "[^a]", "(?:ab)+", "(a(b(c)))", "[a-c-e]", "\\B", "" };
	static const char *txts[] = { "a", "ab", "xay", "yax", "\nx", "x\n", "", "aab", "abcabc", "  a " };
	static const int cf[] = { 0, 1 /*ICASE*/, 2 /*NEWLINE*/, 3 };
	static const int ef[] = { 0, 4 /*NOTBOL*/ };
	int i, j, c, e, k;
	printf("-- regexp api --\n");
	for (i = 0; i < (int)(sizeof pats / sizeof *pats); ++i) {
		for (c = 0; c < 4; ++c) {
			const char *err = NULL;
			Reprog *p = js_regcomp(pats[i], cf[c], &err);
			if (!p) { printf("compile '%s' cf=%d error: %s\n", pats[i], cf[c], err); continue; }
			for (j = 0; j < (int)(sizeof txts / sizeof *txts); ++j) {
				for (e = 0; e < 2; ++e) {
					Resub m;
					memset(&m, 0, sizeof m);
					int r = js_regexec(p, txts[j], &m, ef[e]);
					printf("'%s' cf=%d '%s' ef=%d -> %d nsub=%d", pats[i], cf[c], txts[j], ef[e], r, r ? 0 : m.nsub);
					if (!r) for (k = 0; k < m.nsub; ++k)
						printf(" [%d]=%d..%d", k, m.sub[k].sp ? (int)(m.sub[k].sp - txts[j]) : -1,
							m.sub[k].ep ? (int)(m.sub[k].ep - txts[j]) : -1);
					putchar('\n');
					r = js_regexec(p, txts[j], NULL, ef[e]);
					printf("   nosub -> %d\n", r);
				}
			}
			js_regfree(p);
		}
	}
	printf("-- regexp api done --\n");
}

int main(int argc, char **argv)
{
	js_State *J;
	char *src;
	int flags = 0;
	int i;

	setvbuf(stdout, NULL, _IONBF, 0);
	setvbuf(stderr, NULL, _IONBF, 0);

	for (i = 2; i < argc; ++i)
		if (!strcmp(argv[i], "strict")) flags |= JS_STRICT;

	J = js_newstate(NULL, NULL, flags);
	if (!J) { printf("cannot create state\n"); return 1; }
	setup(J);

	for (i = 2; i < argc; ++i) {
		if (!strcmp(argv[i], "limit")) js_setlimit(J, 2000000, 0);
		if (!strcmp(argv[i], "memlimit")) js_setlimit(J, 0, 1 << 20);
	}

	if (argc > 1 && !strcmp(argv[1], "-api")) {
		apitest(J);
	} else if (argc > 1 && !strcmp(argv[1], "-lowlevel")) {
		lowleveltest(J);
	} else if (argc > 1 && !strcmp(argv[1], "-regexp")) {
		regexptest();
	} else if (argc > 1 && !strcmp(argv[1], "-ctx")) {
		js_setcontext(J, (void*)"context");
		printf("ctx=%s\n", (char*)js_getcontext(J));
		js_pushnumber(J, 1);
		if (js_try(J)) { printf("insert error: %s\n", js_tostring(J, -1)); js_pop(J, 1); }
		else { js_insert(J, 0); js_endtry(J); }
		js_pushstring(J, "iter");
		js_newarray(J);
		js_pushnumber(J, 1); js_setindex(J, -2, 0);
		js_pushiterator(J, -1, 1);
		{ const char *n; while ((n = js_nextiterator(J, -1))) printf("arr iter %s\n", n); }
		js_pop(J, 1);
		js_pushstring(J, "str");
		js_pushiterator(J, -1, 0);
		{ const char *n; while ((n = js_nextiterator(J, -1))) printf("str iter %s\n", n); }
		js_pop(J, 2);
		js_newcfunctionx(J, jsB_print, "withdata", 0, (void*)"DATA", ud_finalize);
		js_setglobal(J, "withdata");
		js_dostring(J, "withdata(1)");
		js_gc(J, 1);
	} else if (argc > 1) {
		src = readfile(argv[1]);
		printf("dostring returned %d\n", js_dostring(J, src));
		free(src);
	}

	js_gc(J, 1);
	for (i = 2; i < argc; ++i)
		if (!strcmp(argv[i], "dumpstrings")) jsS_dumpstrings(J);
	js_freestate(J);
	printf("done\n");
	return 0;
}
