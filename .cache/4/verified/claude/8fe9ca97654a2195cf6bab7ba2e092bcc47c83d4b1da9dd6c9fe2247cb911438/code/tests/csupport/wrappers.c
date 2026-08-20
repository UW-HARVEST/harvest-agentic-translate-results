/*
 * Differential-test support code -- NOT part of the library under test.
 *
 * c_src/inc/q_shared.h implements a large part of the vector maths as
 * function-like macros and `static ID_INLINE` functions.  Those have no
 * external linkage, so they do not appear in the shared library's symbol table
 * and cannot be reached with dlsym().  This file gives every one of them a
 * `w_`-prefixed extern entry point by expanding the *real* header, so that the
 * Rust translation of the header (src/q_shared.rs, exported through
 * src/wrappers.rs) can be compared against the C preprocessor's own expansion.
 *
 * c_src/ is left untouched: this file only #includes it.
 */

#include "q_shared.h"
#include <stddef.h>

/* --- macros ------------------------------------------------------------- */

vec_t w_DotProduct(const vec3_t x, const vec3_t y) { return DotProduct(x, y); }
void w_VectorSubtract(const vec3_t a, const vec3_t b, vec3_t c) { VectorSubtract(a, b, c); }
void w_VectorAdd(const vec3_t a, const vec3_t b, vec3_t c) { VectorAdd(a, b, c); }
void w_VectorCopy(const vec3_t a, vec3_t b) { VectorCopy(a, b); }
void w_VectorScale(const vec3_t v, vec_t s, vec3_t o) { VectorScale(v, s, o); }
void w_VectorMA(const vec3_t v, vec_t s, const vec3_t b, vec3_t o) { VectorMA(v, s, b, o); }
void w_VectorClear(vec3_t a) { VectorClear(a); }
void w_VectorNegate(const vec3_t a, vec3_t b) { VectorNegate(a, b); }
void w_VectorSet(vec3_t v, vec_t x, vec_t y, vec_t z) { VectorSet(v, x, y, z); }
void w_Vector4Copy(const vec4_t a, vec4_t b) { Vector4Copy(a, b); }
void w_SnapVector(vec3_t v) { SnapVector(v); }

int w_IS_NAN(float x) { return IS_NAN(x); }
float w_SQRTFAST(float x) { return SQRTFAST(x); }
double w_DEG2RAD(float a) { return DEG2RAD(a); }
double w_RAD2DEG(float a) { return RAD2DEG(a); }
int w_ANGLE2SHORT(float x) { return ANGLE2SHORT(x); }
double w_SHORT2ANGLE(int x) { return SHORT2ANGLE(x); }
int w_ColorIndex(int c) { return ColorIndex(c); }
float w_Square(float x) { return Square(x); }
int w_PlaneTypeForNormal(const vec3_t x) { return PlaneTypeForNormal(x); }
int w_Q_IsColorString(const char *p) { return Q_IsColorString(p); }
float w_random(void) { return random(); }
float w_crandom(void) { return crandom(); }
void w_MAKERGB(vec3_t v, vec_t r, vec_t g, vec_t b) { MAKERGB(v, r, g, b); }
void w_MAKERGBA(vec4_t v, vec_t r, vec_t g, vec_t b, vec_t a) { MAKERGBA(v, r, g, b, a); }

/* --- static ID_INLINE functions ----------------------------------------- */

int w_VectorCompare(const vec3_t v1, const vec3_t v2) { return VectorCompare(v1, v2); }
vec_t w_VectorLength(const vec3_t v) { return VectorLength(v); }
vec_t w_VectorLengthSquared(const vec3_t v) { return VectorLengthSquared(v); }
vec_t w_Distance(const vec3_t p1, const vec3_t p2) { return Distance(p1, p2); }
vec_t w_DistanceSquared(const vec3_t p1, const vec3_t p2) { return DistanceSquared(p1, p2); }
void w_VectorNormalizeFast(vec3_t v) { VectorNormalizeFast(v); }
void w_VectorInverse(vec3_t v) { VectorInverse(v); }
void w_CrossProduct(const vec3_t v1, const vec3_t v2, vec3_t cross) { CrossProduct(v1, v2, cross); }

/* --- constants and layout ---------------------------------------------- */

void w_layout(int *out) {
	out[0] = (int)sizeof(cplane_t);
	out[1] = (int)offsetof(cplane_t, normal);
	out[2] = (int)offsetof(cplane_t, dist);
	out[3] = (int)offsetof(cplane_t, type);
	out[4] = (int)offsetof(cplane_t, signbits);
	out[5] = (int)offsetof(cplane_t, pad);
	out[6] = NUMVERTEXNORMALS;
	out[7] = nanmask;
	out[8] = (int)sizeof(vec_t);
}

void w_angle_indexes(int *out) {
	out[0] = PITCH;
	out[1] = YAW;
	out[2] = ROLL;
	out[3] = PLANE_X;
	out[4] = PLANE_Y;
	out[5] = PLANE_Z;
	out[6] = PLANE_NON_AXIAL;
	out[7] = qfalse;
	out[8] = qtrue;
	out[9] = (int)sizeof(qboolean);
}

double w_M_PI(void) { return M_PI; }
