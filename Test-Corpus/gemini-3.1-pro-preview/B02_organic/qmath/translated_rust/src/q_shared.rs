pub use crate::surfaceflags::*;

pub type byte = u8;
pub type qboolean = i32;
pub const qfalse: qboolean = 0;
pub const qtrue: qboolean = 1;

pub type qhandle_t = i32;
pub type sfxHandle_t = i32;
pub type fileHandle_t = i32;
pub type clipHandle_t = i32;

pub const MAX_QINT: i32 = 0x7fffffff;
pub const MIN_QINT: i32 = -MAX_QINT - 1;

pub const PITCH: usize = 0;
pub const YAW: usize = 1;
pub const ROLL: usize = 2;

pub const MAX_STRING_CHARS: usize = 1024;
pub const MAX_STRING_TOKENS: usize = 1024;
pub const MAX_TOKEN_CHARS: usize = 1024;
pub const MAX_INFO_STRING: usize = 1024;
pub const MAX_INFO_KEY: usize = 1024;
pub const MAX_INFO_VALUE: usize = 1024;
pub const BIG_INFO_STRING: usize = 8192;
pub const BIG_INFO_KEY: usize = 8192;
pub const BIG_INFO_VALUE: usize = 8192;
pub const MAX_QPATH: usize = 64;
pub const MAX_OSPATH: usize = 256;
pub const MAX_NAME_LENGTH: usize = 32;
pub const MAX_SAY_TEXT: usize = 150;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum cbufExec_t {
    EXEC_NOW,
    EXEC_INSERT,
    EXEC_APPEND,
}

pub const MAX_MAP_AREA_BYTES: usize = 32;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum printParm_t {
    PRINT_ALL,
    PRINT_DEVELOPER,
    PRINT_WARNING,
    PRINT_ERROR,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum errorParm_t {
    ERR_FATAL,
    ERR_DROP,
    ERR_SERVERDISCONNECT,
    ERR_DISCONNECT,
    ERR_NEED_CD,
}

pub const PROP_GAP_WIDTH: i32 = 3;
pub const PROP_SPACE_WIDTH: i32 = 8;
pub const PROP_HEIGHT: i32 = 27;
pub const PROP_SMALL_SIZE_SCALE: f32 = 0.75;
pub const BLINK_DIVISOR: i32 = 200;
pub const PULSE_DIVISOR: i32 = 75;

pub const UI_LEFT: u32 = 0x00000000;
pub const UI_CENTER: u32 = 0x00000001;
pub const UI_RIGHT: u32 = 0x00000002;
pub const UI_FORMATMASK: u32 = 0x00000007;
pub const UI_SMALLFONT: u32 = 0x00000010;
pub const UI_BIGFONT: u32 = 0x00000020;
pub const UI_GIANTFONT: u32 = 0x00000040;
pub const UI_DROPSHADOW: u32 = 0x00000800;
pub const UI_BLINK: u32 = 0x00001000;
pub const UI_INVERSE: u32 = 0x00002000;
pub const UI_PULSE: u32 = 0x00004000;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ha_pref {
    h_high,
    h_low,
    h_dontcare,
}

pub const CIN_system: i32 = 1;
pub const CIN_loop: i32 = 2;
pub const CIN_hold: i32 = 4;
pub const CIN_silent: i32 = 8;
pub const CIN_shader: i32 = 16;

pub type vec_t = f32;
pub type vec2_t = [vec_t; 2];
pub type vec3_t = [vec_t; 3];
pub type vec4_t = [vec_t; 4];
pub type vec5_t = [vec_t; 5];

pub type fixed4_t = i32;
pub type fixed8_t = i32;
pub type fixed16_t = i32;

pub const M_PI: f32 = std::f32::consts::PI;

pub const NUMVERTEXNORMALS: usize = 162;

pub const SCREEN_WIDTH: i32 = 640;
pub const SCREEN_HEIGHT: i32 = 480;

pub const TINYCHAR_WIDTH: i32 = SMALLCHAR_WIDTH;
pub const TINYCHAR_HEIGHT: i32 = SMALLCHAR_HEIGHT / 2;
pub const SMALLCHAR_WIDTH: i32 = 8;
pub const SMALLCHAR_HEIGHT: i32 = 16;
pub const BIGCHAR_WIDTH: i32 = 16;
pub const BIGCHAR_HEIGHT: i32 = 16;
pub const GIANTCHAR_WIDTH: i32 = 32;
pub const GIANTCHAR_HEIGHT: i32 = 48;

pub const Q_COLOR_ESCAPE: char = '^';
pub const COLOR_BLACK: char = '0';
pub const COLOR_RED: char = '1';
pub const COLOR_GREEN: char = '2';
pub const COLOR_YELLOW: char = '3';
pub const COLOR_BLUE: char = '4';
pub const COLOR_CYAN: char = '5';
pub const COLOR_MAGENTA: char = '6';
pub const COLOR_WHITE: char = '7';

pub const S_COLOR_BLACK: &str = "^0";
pub const S_COLOR_RED: &str = "^1";
pub const S_COLOR_GREEN: &str = "^2";
pub const S_COLOR_YELLOW: &str = "^3";
pub const S_COLOR_BLUE: &str = "^4";
pub const S_COLOR_CYAN: &str = "^5";
pub const S_COLOR_MAGENTA: &str = "^6";
pub const S_COLOR_WHITE: &str = "^7";

pub fn DEG2RAD(a: f32) -> f32 { a * M_PI / 180.0 }
pub fn RAD2DEG(a: f32) -> f32 { a * 180.0 / M_PI }

pub fn DotProduct(x: &vec3_t, y: &vec3_t) -> f32 {
    x[0] * y[0] + x[1] * y[1] + x[2] * y[2]
}

pub fn VectorSubtract(a: &vec3_t, b: &vec3_t, c: &mut vec3_t) {
    c[0] = a[0] - b[0];
    c[1] = a[1] - b[1];
    c[2] = a[2] - b[2];
}

pub fn VectorAdd(a: &vec3_t, b: &vec3_t, c: &mut vec3_t) {
    c[0] = a[0] + b[0];
    c[1] = a[1] + b[1];
    c[2] = a[2] + b[2];
}

pub fn VectorCopy(a: &vec3_t, b: &mut vec3_t) {
    b[0] = a[0];
    b[1] = a[1];
    b[2] = a[2];
}

pub fn VectorScale(v: &vec3_t, s: f32, o: &mut vec3_t) {
    o[0] = v[0] * s;
    o[1] = v[1] * s;
    o[2] = v[2] * s;
}

pub fn VectorMA(v: &vec3_t, s: f32, b: &vec3_t, o: &mut vec3_t) {
    o[0] = v[0] + b[0] * s;
    o[1] = v[1] + b[1] * s;
    o[2] = v[2] + b[2] * s;
}

pub fn VectorClear(a: &mut vec3_t) {
    a[0] = 0.0;
    a[1] = 0.0;
    a[2] = 0.0;
}

pub fn VectorNegate(a: &vec3_t, b: &mut vec3_t) {
    b[0] = -a[0];
    b[1] = -a[1];
    b[2] = -a[2];
}

pub fn VectorSet(v: &mut vec3_t, x: f32, y: f32, z: f32) {
    v[0] = x;
    v[1] = y;
    v[2] = z;
}

pub fn Vector4Copy(a: &vec4_t, b: &mut vec4_t) {
    b[0] = a[0];
    b[1] = a[1];
    b[2] = a[2];
    b[3] = a[3];
}

pub fn SnapVector(v: &mut vec3_t) {
    v[0] = v[0].trunc();
    v[1] = v[1].trunc();
    v[2] = v[2].trunc();
}

pub fn VectorCompare(v1: &vec3_t, v2: &vec3_t) -> i32 {
    if v1[0] != v2[0] || v1[1] != v2[1] || v1[2] != v2[2] {
        0
    } else {
        1
    }
}

pub fn VectorLength(v: &vec3_t) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

pub fn VectorLengthSquared(v: &vec3_t) -> f32 {
    v[0] * v[0] + v[1] * v[1] + v[2] * v[2]
}

pub fn Distance(p1: &vec3_t, p2: &vec3_t) -> f32 {
    let mut v = [0.0; 3];
    VectorSubtract(p2, p1, &mut v);
    VectorLength(&v)
}

pub fn DistanceSquared(p1: &vec3_t, p2: &vec3_t) -> f32 {
    let mut v = [0.0; 3];
    VectorSubtract(p2, p1, &mut v);
    v[0] * v[0] + v[1] * v[1] + v[2] * v[2]
}

pub fn VectorNormalizeFast(v: &mut vec3_t) {
    let ilength = crate::q_math::Q_rsqrt(DotProduct(v, v));
    v[0] *= ilength;
    v[1] *= ilength;
    v[2] *= ilength;
}

pub fn VectorInverse(v: &mut vec3_t) {
    v[0] = -v[0];
    v[1] = -v[1];
    v[2] = -v[2];
}

pub fn CrossProduct(v1: &vec3_t, v2: &vec3_t, cross: &mut vec3_t) {
    cross[0] = v1[1] * v2[2] - v1[2] * v2[1];
    cross[1] = v1[2] * v2[0] - v1[0] * v2[2];
    cross[2] = v1[0] * v2[1] - v1[1] * v2[0];
}

#[derive(Copy, Clone)]
pub struct pc_token_t {
    pub type_: i32,
    pub subtype: i32,
    pub intvalue: i32,
    pub floatvalue: f32,
    pub string: [u8; 1024],
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum fsMode_t {
    FS_READ,
    FS_WRITE,
    FS_APPEND,
    FS_APPEND_SYNC,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum fsOrigin_t {
    FS_SEEK_CUR,
    FS_SEEK_END,
    FS_SEEK_SET,
}

#[derive(Copy, Clone)]
pub struct qint64 {
    pub b0: byte,
    pub b1: byte,
    pub b2: byte,
    pub b3: byte,
    pub b4: byte,
    pub b5: byte,
    pub b6: byte,
    pub b7: byte,
}

pub const CVAR_ARCHIVE: i32 = 1;
pub const CVAR_USERINFO: i32 = 2;
pub const CVAR_SERVERINFO: i32 = 4;
pub const CVAR_SYSTEMINFO: i32 = 8;
pub const CVAR_INIT: i32 = 16;
pub const CVAR_LATCH: i32 = 32;
pub const CVAR_ROM: i32 = 64;
pub const CVAR_USER_CREATED: i32 = 128;
pub const CVAR_TEMP: i32 = 256;
pub const CVAR_CHEAT: i32 = 512;
pub const CVAR_NORESTART: i32 = 1024;

pub const MAX_CVAR_VALUE_STRING: usize = 256;
pub type cvarHandle_t = i32;

#[derive(Copy, Clone)]
pub struct vmCvar_t {
    pub handle: cvarHandle_t,
    pub modificationCount: i32,
    pub value: f32,
    pub integer: i32,
    pub string: [u8; MAX_CVAR_VALUE_STRING],
}

pub const PLANE_X: u8 = 0;
pub const PLANE_Y: u8 = 1;
pub const PLANE_Z: u8 = 2;
pub const PLANE_NON_AXIAL: u8 = 3;

#[derive(Copy, Clone)]
pub struct cplane_t {
    pub normal: vec3_t,
    pub dist: f32,
    pub type_: u8,
    pub signbits: u8,
    pub pad: [u8; 2],
}

#[derive(Copy, Clone)]
pub struct trace_t {
    pub allsolid: qboolean,
    pub startsolid: qboolean,
    pub fraction: f32,
    pub endpos: vec3_t,
    pub plane: cplane_t,
    pub surfaceFlags: i32,
    pub contents: i32,
    pub entityNum: i32,
}

#[derive(Copy, Clone)]
pub struct markFragment_t {
    pub firstPoint: i32,
    pub numPoints: i32,
}

#[derive(Copy, Clone)]
pub struct orientation_t {
    pub origin: vec3_t,
    pub axis: [vec3_t; 3],
}

pub const KEYCATCH_CONSOLE: i32 = 0x0001;
pub const KEYCATCH_UI: i32 = 0x0002;
pub const KEYCATCH_MESSAGE: i32 = 0x0004;
pub const KEYCATCH_CGAME: i32 = 0x0008;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum soundChannel_t {
    CHAN_AUTO,
    CHAN_LOCAL,
    CHAN_WEAPON,
    CHAN_VOICE,
    CHAN_ITEM,
    CHAN_BODY,
    CHAN_LOCAL_SOUND,
    CHAN_ANNOUNCER,
}

pub const SNAPFLAG_RATE_DELAYED: i32 = 1;
pub const SNAPFLAG_NOT_ACTIVE: i32 = 2;
pub const SNAPFLAG_SERVERCOUNT: i32 = 4;

pub const MAX_CLIENTS: usize = 64;
pub const MAX_LOCATIONS: usize = 64;
pub const GENTITYNUM_BITS: usize = 10;
pub const MAX_GENTITIES: usize = 1 << GENTITYNUM_BITS;
pub const ENTITYNUM_NONE: usize = MAX_GENTITIES - 1;
pub const ENTITYNUM_WORLD: usize = MAX_GENTITIES - 2;
pub const ENTITYNUM_MAX_NORMAL: usize = MAX_GENTITIES - 2;

pub const MAX_MODELS: usize = 256;
pub const MAX_SOUNDS: usize = 256;
pub const MAX_CONFIGSTRINGS: usize = 1024;
pub const CS_SERVERINFO: usize = 0;
pub const CS_SYSTEMINFO: usize = 1;
pub const RESERVED_CONFIGSTRINGS: usize = 2;
pub const MAX_GAMESTATE_CHARS: usize = 16000;

#[derive(Copy, Clone)]
pub struct gameState_t {
    pub stringOffsets: [i32; MAX_CONFIGSTRINGS],
    pub stringData: [u8; MAX_GAMESTATE_CHARS],
    pub dataCount: i32,
}

pub const MAX_STATS: usize = 16;
pub const MAX_PERSISTANT: usize = 16;
pub const MAX_POWERUPS: usize = 16;
pub const MAX_WEAPONS: usize = 16;
pub const MAX_PS_EVENTS: usize = 2;
pub const PS_PMOVEFRAMECOUNTBITS: usize = 6;

#[derive(Copy, Clone)]
pub struct playerState_t {
    pub commandTime: i32,
    pub pm_type: i32,
    pub bobCycle: i32,
    pub pm_flags: i32,
    pub pm_time: i32,
    pub origin: vec3_t,
    pub velocity: vec3_t,
    pub weaponTime: i32,
    pub gravity: i32,
    pub speed: i32,
    pub delta_angles: [i32; 3],
    pub groundEntityNum: i32,
    pub legsTimer: i32,
    pub legsAnim: i32,
    pub torsoTimer: i32,
    pub torsoAnim: i32,
    pub movementDir: i32,
    pub grapplePoint: vec3_t,
    pub eFlags: i32,
    pub eventSequence: i32,
    pub events: [i32; MAX_PS_EVENTS],
    pub eventParms: [i32; MAX_PS_EVENTS],
    pub externalEvent: i32,
    pub externalEventParm: i32,
    pub externalEventTime: i32,
    pub clientNum: i32,
    pub weapon: i32,
    pub weaponstate: i32,
    pub viewangles: vec3_t,
    pub viewheight: i32,
    pub damageEvent: i32,
    pub damageYaw: i32,
    pub damagePitch: i32,
    pub damageCount: i32,
    pub stats: [i32; MAX_STATS],
    pub persistant: [i32; MAX_PERSISTANT],
    pub powerups: [i32; MAX_POWERUPS],
    pub ammo: [i32; MAX_WEAPONS],
    pub generic1: i32,
    pub loopSound: i32,
    pub jumppad_ent: i32,
    pub ping: i32,
    pub pmove_framecount: i32,
    pub jumppad_frame: i32,
    pub entityEventSequence: i32,
}

pub const BUTTON_ATTACK: i32 = 1;
pub const BUTTON_TALK: i32 = 2;
pub const BUTTON_USE_HOLDABLE: i32 = 4;
pub const BUTTON_GESTURE: i32 = 8;
pub const BUTTON_WALKING: i32 = 16;
pub const BUTTON_AFFIRMATIVE: i32 = 32;
pub const BUTTON_NEGATIVE: i32 = 64;
pub const BUTTON_GETFLAG: i32 = 128;
pub const BUTTON_GUARDBASE: i32 = 256;
pub const BUTTON_PATROL: i32 = 512;
pub const BUTTON_FOLLOWME: i32 = 1024;
pub const BUTTON_ANY: i32 = 2048;
pub const MOVE_RUN: i32 = 120;

#[derive(Copy, Clone)]
pub struct usercmd_t {
    pub serverTime: i32,
    pub angles: [i32; 3],
    pub buttons: i32,
    pub weapon: byte,
    pub forwardmove: i8,
    pub rightmove: i8,
    pub upmove: i8,
}

pub const SOLID_BMODEL: i32 = 0xffffff;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum trType_t {
    TR_STATIONARY,
    TR_INTERPOLATE,
    TR_LINEAR,
    TR_LINEAR_STOP,
    TR_SINE,
    TR_GRAVITY,
}

#[derive(Copy, Clone)]
pub struct trajectory_t {
    pub trType: trType_t,
    pub trTime: i32,
    pub trDuration: i32,
    pub trBase: vec3_t,
    pub trDelta: vec3_t,
}

#[derive(Copy, Clone)]
pub struct entityState_t {
    pub number: i32,
    pub eType: i32,
    pub eFlags: i32,
    pub pos: trajectory_t,
    pub apos: trajectory_t,
    pub time: i32,
    pub time2: i32,
    pub origin: vec3_t,
    pub origin2: vec3_t,
    pub angles: vec3_t,
    pub angles2: vec3_t,
    pub otherEntityNum: i32,
    pub otherEntityNum2: i32,
    pub groundEntityNum: i32,
    pub constantLight: i32,
    pub loopSound: i32,
    pub modelindex: i32,
    pub modelindex2: i32,
    pub clientNum: i32,
    pub frame: i32,
    pub solid: i32,
    pub event: i32,
    pub eventParm: i32,
    pub powerups: i32,
    pub weapon: i32,
    pub legsAnim: i32,
    pub torsoAnim: i32,
    pub generic1: i32,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum connstate_t {
    CA_UNINITIALIZED,
    CA_DISCONNECTED,
    CA_AUTHORIZING,
    CA_CONNECTING,
    CA_CHALLENGING,
    CA_CONNECTED,
    CA_LOADING,
    CA_PRIMED,
    CA_ACTIVE,
    CA_CINEMATIC,
}

pub const GLYPH_START: usize = 0;
pub const GLYPH_END: usize = 255;
pub const GLYPH_CHARSTART: usize = 32;
pub const GLYPH_CHAREND: usize = 127;
pub const GLYPHS_PER_FONT: usize = GLYPH_END - GLYPH_START + 1;

#[derive(Copy, Clone)]
pub struct glyphInfo_t {
    pub height: i32,
    pub top: i32,
    pub bottom: i32,
    pub pitch: i32,
    pub xSkip: i32,
    pub imageWidth: i32,
    pub imageHeight: i32,
    pub s: f32,
    pub t: f32,
    pub s2: f32,
    pub t2: f32,
    pub glyph: qhandle_t,
    pub shaderName: [u8; 32],
}

#[derive(Copy, Clone)]
pub struct fontInfo_t {
    pub glyphs: [glyphInfo_t; GLYPHS_PER_FONT],
    pub glyphScale: f32,
    pub name: [u8; MAX_QPATH],
}

#[derive(Copy, Clone)]
pub struct qtime_t {
    pub tm_sec: i32,
    pub tm_min: i32,
    pub tm_hour: i32,
    pub tm_mday: i32,
    pub tm_mon: i32,
    pub tm_year: i32,
    pub tm_wday: i32,
    pub tm_yday: i32,
    pub tm_isdst: i32,
}

pub const AS_LOCAL: i32 = 0;
pub const AS_MPLAYER: i32 = 1;
pub const AS_GLOBAL: i32 = 2;
pub const AS_FAVORITES: i32 = 3;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum e_status {
    FMV_IDLE,
    FMV_PLAY,
    FMV_EOF,
    FMV_ID_BLT,
    FMV_ID_IDLE,
    FMV_LOOPED,
    FMV_ID_WAIT,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum flagStatus_t {
    FLAG_ATBASE = 0,
    FLAG_TAKEN,
    FLAG_TAKEN_RED,
    FLAG_TAKEN_BLUE,
    FLAG_DROPPED,
}

pub const MAX_GLOBAL_SERVERS: usize = 4096;
pub const MAX_OTHER_SERVERS: usize = 128;
pub const MAX_PINGREQUESTS: usize = 32;
pub const MAX_SERVERSTATUSREQUESTS: usize = 16;

pub const SAY_ALL: i32 = 0;
pub const SAY_TEAM: i32 = 1;
pub const SAY_TELL: i32 = 2;

pub const CDKEY_LEN: usize = 16;
pub const CDCHKSUM_LEN: usize = 2;
