use crate::context::SpxCtx;

extern "C" {
    #[link_name = "SPX_initialize_hash_function"]
    pub fn initialize_hash_function(ctx: *mut SpxCtx);

    #[link_name = "SPX_prf_addr"]
    pub fn prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32);

    #[link_name = "SPX_gen_message_random"]
    pub fn gen_message_random(
        r: *mut u8,
        sk_prf: *const u8,
        optrand: *const u8,
        m: *const u8,
        mlen: u64,
        ctx: *const SpxCtx,
    );

    #[link_name = "SPX_hash_message"]
    pub fn hash_message(
        digest: *mut u8,
        tree: *mut u64,
        leaf_idx: *mut u32,
        r: *const u8,
        pk: *const u8,
        m: *const u8,
        mlen: u64,
        ctx: *const SpxCtx,
    );
}
