#include <stdio.h>
#include <stddef.h>
#define ZSTD_STATIC_LINKING_ONLY
#define FSE_STATIC_LINKING_ONLY
#define XXH_STATIC_LINKING_ONLY
#include "zstd.h"
#include "common/zstd_internal.h"
#include "compress/zstd_compress_internal.h"
#include "decompress/zstd_decompress_internal.h"
#include "compress/zstd_cwksp.h"

struct ZSTD_CDict_s {
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
struct ZSTD_DDict_s {
    void* dictBuffer;
    const void* dictContent;
    size_t dictSize;
    ZSTD_entropyDTables_t entropy;
    U32 dictID;
    U32 entropyPresent;
    ZSTD_customMem cMem;
};

#define P(t) printf("%s %zu %zu\n", #t, sizeof(t), _Alignof(t))
int main(void) {
    P(ZSTD_cwksp);
    P(SeqDef); P(SeqStore_t); P(ZSTD_window_t); P(optState_t);
    P(ZSTD_MatchState_t); P(ZSTD_blockState_t);
    P(ZSTD_hufCTables_t); P(ZSTD_fseCTables_t); P(ZSTD_entropyCTables_t);
    P(ZSTD_hufCTablesMetadata_t); P(ZSTD_fseCTablesMetadata_t); P(ZSTD_entropyCTablesMetadata_t);
    P(ldmEntry_t); P(ldmMatchCandidate_t); P(ldmState_t); P(ldmParams_t);
    P(SeqCollector); P(ZSTD_CCtx_params); P(ZSTD_blockSplitCtx); P(ZSTD_CCtx);
    P(struct ZSTD_CDict_s);
    P(ZSTD_seqSymbol); P(ZSTD_entropyDTables_t); P(ZSTD_DCtx);
    P(struct ZSTD_DDict_s);
    P(ZSTD_compressedBlockState_t);
    P(XXH64_state_t); P(XXH32_state_t);
    P(RawSeqStore_t); P(rawSeq); P(ZSTD_match_t); P(ZSTD_optimal_t);
    P(ZSTD_DDictHashSet);
    printf("OFF_CCtx_workspace %zu\n", offsetof(ZSTD_CCtx, workspace));
    printf("OFF_CCtx_blockState %zu\n", offsetof(ZSTD_CCtx, blockState));
    printf("OFF_CCtx_blockSplitCtx %zu\n", offsetof(ZSTD_CCtx, blockSplitCtx));
    printf("OFF_DCtx_entropy %zu\n", offsetof(ZSTD_DCtx, entropy));
    printf("OFF_DCtx_litExtraBuffer %zu\n", offsetof(ZSTD_DCtx, litExtraBuffer));
    printf("OFF_DCtx_oversizedDuration %zu\n", offsetof(ZSTD_DCtx, oversizedDuration));
    printf("OFF_MS_opt %zu\n", offsetof(ZSTD_MatchState_t, opt));
    printf("OFF_MS_cParams %zu\n", offsetof(ZSTD_MatchState_t, cParams));
    return 0;
}
