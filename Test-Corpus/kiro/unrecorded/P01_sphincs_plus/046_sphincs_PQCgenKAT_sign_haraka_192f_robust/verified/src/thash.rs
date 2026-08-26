use crate::context::SpxCtx;

extern "C" {
    #[link_name = "SPX_thash"]
    pub fn thash(
        out: *mut u8,
        inp: *const u8,
        inblocks: u32,
        ctx: *const SpxCtx,
        addr: *mut u32,
    );
}
