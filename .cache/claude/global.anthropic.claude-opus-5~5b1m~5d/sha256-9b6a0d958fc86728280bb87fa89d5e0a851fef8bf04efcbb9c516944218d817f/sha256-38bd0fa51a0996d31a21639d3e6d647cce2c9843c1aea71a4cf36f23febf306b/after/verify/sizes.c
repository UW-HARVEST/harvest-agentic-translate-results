#include <stdio.h>
#include <stddef.h>
#define ZSTD_STATIC_LINKING_ONLY
#define FSE_STATIC_LINKING_ONLY
#define HUF_STATIC_LINKING_ONLY
#define XXH_STATIC_LINKING_ONLY
#include "zstd.h"
#include "compress/zstd_compress_internal.h"
#include "decompress/zstd_decompress_internal.h"
#include "compress/zstdmt_compress.h"

struct MY_CDict_s {
    const void* dictContent;
    size_t dictContentSize;
    ZSTD_dictContentType_e dictContentType;
    U32* entropyWorkspace;
    ZSTD_cwksp workspace;
    ZSTD_MatchState_t matchState;
    ZSTD_compressedBlockState_t cBlockState;
    ZSTD_customMem customMem;
    U32 dictID;
    int compressionLevel;
    ZSTD_ParamSwitch_e useRowMatchFinder;
};
struct MY_DDict_s {
    void* dictBuffer;
    const void* dictContent;
    size_t dictSize;
    ZSTD_entropyDTables_t entropy;
    U32 dictID;
    U32 entropyPresent;
    ZSTD_customMem cMem;
};
int main(void) {
    printf("%-40s %zu\n", "ZSTD_CDict", sizeof(struct MY_CDict_s));
    printf("%-40s %zu\n", "ZSTD_DDict", sizeof(struct MY_DDict_s));
    printf("%-40s %zu\n", "off CDict.matchState", offsetof(struct MY_CDict_s, matchState));
    printf("%-40s %zu\n", "off CDict.cBlockState", offsetof(struct MY_CDict_s, cBlockState));
    printf("%-40s %zu\n", "off CDict.useRowMatchFinder", offsetof(struct MY_CDict_s, useRowMatchFinder));
#define P(T) printf("%-40s %zu\n", #T, sizeof(T))
    P(ZSTD_CCtx);
    P(ZSTD_DCtx);
    P(ZSTD_CCtx_params);
    P(ZSTD_cwksp);
    P(ZSTD_MatchState_t);
    P(ZSTD_compressedBlockState_t);
    P(ZSTD_entropyCTables_t);
    P(ZSTD_hufCTables_t);
    P(ZSTD_fseCTables_t);
    P(SeqStore_t);
    P(SeqDef);
    P(ldmState_t);
    P(ldmParams_t);
    P(optState_t);
    P(ZSTD_blockSplitCtx);
    P(ZSTD_entropyCTablesMetadata_t);
    P(ZSTD_hufCTablesMetadata_t);
    P(ZSTD_fseCTablesMetadata_t);
    P(ZSTD_window_t);
    P(ZSTD_blockState_t);
    P(RawSeqStore_t);
    P(SeqCollector);
    P(ZSTD_localDict);
    P(ZSTD_prefixDict);
    P(XXH64_state_t);
    P(XXH32_state_t);
    P(ZSTD_optimal_t);
    P(ZSTD_match_t);
    printf("--- decompress ---\n");
    P(ZSTD_entropyDTables_t);
    P(ZSTD_seqSymbol);
    P(ZSTD_FrameHeader);
    printf("--- offsets CCtx ---\n");
    printf("%-40s %zu\n", "off blockSplitCtx", offsetof(ZSTD_CCtx, blockSplitCtx));
    printf("%-40s %zu\n", "off extSeqBuf", offsetof(ZSTD_CCtx, extSeqBuf));
    printf("%-40s %zu\n", "off workspace", offsetof(ZSTD_CCtx, workspace));
    printf("%-40s %zu\n", "off seqStore", offsetof(ZSTD_CCtx, seqStore));
    printf("%-40s %zu\n", "off ldmState", offsetof(ZSTD_CCtx, ldmState));
    printf("%-40s %zu\n", "off blockState", offsetof(ZSTD_CCtx, blockState));
    printf("--- offsets DCtx ---\n");
    printf("%-40s %zu\n", "off entropy", offsetof(ZSTD_DCtx, entropy));
    printf("%-40s %zu\n", "off litExtraBuffer", offsetof(ZSTD_DCtx, litExtraBuffer));
    printf("%-40s %zu\n", "off headerBuffer", offsetof(ZSTD_DCtx, headerBuffer));
    printf("%-40s %zu\n", "off oversizedDuration", offsetof(ZSTD_DCtx, oversizedDuration));
    return 0;
}
