#include "mujs.h"

#include <stdio.h>

int jsU_chartorune(int *rune, const char *str);
int jsU_runetochar(char *str, const int *rune);
int jsU_tolowerrune(int rune);
int jsU_toupperrune(int rune);

static const char *scripts[] = {
	"emit('arith', 1 + 2 * 3, Math.pow(2, 10), isNaN(NaN), isFinite(Infinity));",
	"var a=[3,1,4,1,5]; emit('array', a.sort().join(':'), a.map(function(x){return x*x}).join(','));",
	"var o={z:1,a:[true,null,'x']}; emit('json', JSON.stringify(o), JSON.parse('{\"n\":12}').n);",
	"var r=/(ab+)(c?)/gi; emit('regexp', 'xxABBCyyabb'.replace(r, '<$1:$2>'), /foo/i.test('FOO'));",
	"emit('string', 'Stra\\u00dfe'.toUpperCase(), '\\u03a3'.toLowerCase(), 'abc'.slice(-2));",
	"emit('date', new Date(Date.UTC(2001,1,3,4,5,6,7)).toISOString());",
	"function C(x){this.x=x} C.prototype.y=9; var c=new C(4); emit('object', c.x+c.y, c instanceof C);",
	"emit('uri', encodeURIComponent('a b/c?'), decodeURIComponent('a%20b%2Fc%3F'));",
	"try { null.x } catch (e) { emit('error', e.name, e instanceof TypeError) }",
	0
};

static void emit(js_State *J)
{
	int i;
	int top = js_gettop(J);

	for (i = 1; i < top; ++i)
		printf("%s%s", i == 1 ? "" : "|", js_tostring(J, i));
	putchar('\n');
}

int main(void)
{
	js_State *J = js_newstate(NULL, NULL, 0);
	int i;
	int rune;
	char encoded[8] = {0};

	if (!J)
		return 2;

	js_newcfunction(J, emit, "emit", 0);
	js_setglobal(J, "emit");

	for (i = 0; scripts[i]; ++i)
		printf("rc:%d\n", js_dostring(J, scripts[i]));

	js_pushundefined(J);
	js_pushnull(J);
	js_pushboolean(J, 1);
	js_pushnumber(J, -0.0);
	js_pushstring(J, "hello");
	printf("stack:%d:%d:%d:%d:%d:%s\n",
		js_gettop(J),
		js_isundefined(J, 0),
		js_isnull(J, 1),
		js_toboolean(J, 2),
		js_type(J, 3),
		js_tostring(J, 4));
	js_pop(J, 5);

	printf("utf:%d:", jsU_chartorune(&rune, "\342\202\254"));
	printf("%x:%d:", rune, jsU_runetochar(encoded, &rune));
	printf("%02x%02x%02x:%x:%x\n",
		(unsigned char)encoded[0],
		(unsigned char)encoded[1],
		(unsigned char)encoded[2],
		jsU_tolowerrune('Q'),
		jsU_toupperrune('q'));

	js_freestate(J);
	return 0;
}
