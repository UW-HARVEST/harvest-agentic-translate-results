#include <stdio.h>
#include <string.h>
#include "../src/closed_syncmers.h"
#include "../src/closed_syncmers_naive.h"

static void run_case(const char *seq, int K, int S, const char *label) {
    int len = (int)strlen(seq);
    int num_results = 0;
    MinimizerResult results[2000];
    compute_closed_syncmers(seq, len, K, S, results, &num_results);

    int num_naive = 0;
    MinimizerResult naive[2000];
    compute_closed_syncmers_naive(seq, len, K, S, naive, &num_naive);

    printf("=== %s seq_len=%d K=%d S=%d ===\n", label, len, K, S);
    printf("FAST count=%d\n", num_results);
    for (int i = 0; i < num_results; i++) {
        printf("  i=%d kmer=%zu smer=%zu hash=%llu\n", i, results[i].kmer_position,
               results[i].smer_position, (unsigned long long)results[i].minimizer_hash);
    }
    printf("NAIVE count=%d\n", num_naive);
    for (int i = 0; i < num_naive; i++) {
        printf("  i=%d kmer=%zu smer=%zu hash=%llu\n", i, naive[i].kmer_position,
               naive[i].smer_position, (unsigned long long)naive[i].minimizer_hash);
    }
}

int main() {
    // Case 1: small fixed sequence
    run_case("ACGTACGTAC", 5, 2, "case1");
    // Case 2: simple
    run_case("AAAAAAAAAA", 5, 2, "case2_all_A");
    // Case 3: alternating
    run_case("ACGTACGTACGTACGTACGT", 6, 3, "case3");
    // Case 4: longer
    run_case("ACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGT", 10, 4, "case4");
    // Case 5: typical k/s
    run_case("ACGTGGCCAATTACGTAGCTAGCTACGATCGAT", 8, 3, "case5");
    // Case 6: varied bases
    run_case("ACGTACGTACGTACGTACGTACGTACGTACGT", 7, 3, "case6");
    // Case 7: K equals length
    run_case("ACGTACGT", 8, 3, "case7_k_eq_len");
    // Case 8: K = S
    run_case("ACGTACGTAC", 4, 4, "case8_k_eq_s");
    // Case 9: random-ish
    run_case("GATTACAGATTACAGATTACA", 7, 3, "case9");
    return 0;
}
