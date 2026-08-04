// Little driver to compute expected values for Rust tests
#include <stdio.h>
#include <stdlib.h>
#include <inttypes.h>
#include <string.h>
#include <float.h>
#include "fst.h"
#include "sr.h"
#include "bitset.h"
#include "queue.h"
#include "heap.h"
#include "symt.h"
#include "iter.h"

int main(int argc, char ** argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: %s <test>\n", argv[0]);
        return 1;
    }

    if (strcmp(argv[1], "sr") == 0) {
        printf("real_sum 1.0 2.0 = %.6f\n", real_sum(1.0, 2.0));
        printf("real_product 3.0 4.0 = %.6f\n", real_product(3.0, 4.0));
        printf("tropical_sum 1.0 2.0 = %.6f\n", tropical_sum(1.0, 2.0));
        printf("tropical_sum 5.0 2.0 = %.6f\n", tropical_sum(5.0, 2.0));
        printf("tropical_product 3.0 4.0 = %.6f\n", tropical_product(3.0, 4.0));
        struct _sr trop = sr_get(SR_TROPICAL);
        struct _sr real = sr_get(SR_REAL);
        printf("trop.zero=%g trop.one=%g\n", trop.zero, trop.one);
        printf("real.zero=%g real.one=%g\n", real.zero, real.one);
    } else if (strcmp(argv[1], "bitset") == 0) {
        struct _bitset * bs = bitset_create(64);
        bitset_set(bs, 5);
        bitset_set(bs, 33);
        printf("bs.get(5)=%d\n", bitset_get(bs, 5));
        printf("bs.get(33)=%d\n", bitset_get(bs, 33));
        printf("bs.get(7)=%d\n", bitset_get(bs, 7));
        printf("bs.n_words=%d\n", bs->n_words);
        bitset_clear(bs, 5);
        printf("after clear bs.get(5)=%d\n", bitset_get(bs, 5));
        bitset_set_all(bs);
        printf("after set_all bs.get(0)=%d bs.get(63)=%d\n", bitset_get(bs, 0), bitset_get(bs, 63));
        bitset_clear_all(bs);
        printf("after clear_all bs.get(0)=%d\n", bitset_get(bs, 0));
        bitset_remove(bs);
    } else if (strcmp(argv[1], "queue") == 0) {
        struct _queue * q = queue_create(sizeof(int));
        int x = 10;
        queue_enque(q, &x);
        x = 20;
        queue_enque(q, &x);
        x = 30;
        queue_enque(q, &x);
        printf("n_items=%zu\n", q->n_items);
        int v;
        queue_deque(q, &v);
        printf("deque: %d\n", v);
        queue_deque(q, &v);
        printf("deque: %d\n", v);
        printf("n_items=%zu\n", q->n_items);
        queue_remove(q);
    } else if (strcmp(argv[1], "fst") == 0) {
        struct _fst * fst = fst_create();
        printf("initial sr_type=%d n_states=%d start=%d\n", fst->sr_type, fst->n_states, fst->start);
        state_t s0 = fst_add_state(fst);
        state_t s1 = fst_add_state(fst);
        state_t s2 = fst_add_state(fst);
        printf("added states %d %d %d\n", s0, s1, s2);
        fst_add_arc(fst, s0, s1, 1, 2, 0.5f);
        fst_add_arc(fst, s0, s2, 3, 4, 1.5f);
        fst_add_arc(fst, s1, s2, 5, 6, 2.5f);
        fst_set_final(fst, s2, 0.0f);
        printf("n_arcs(0)=%d n_arcs(1)=%d n_arcs(2)=%d\n",
            fst->states[0].n_arcs, fst->states[1].n_arcs, fst->states[2].n_arcs);
        printf("get_n_arcs=%d\n", fst_get_n_arcs(fst));
        printf("final[0]=%d final[1]=%d final[2]=%d\n",
            fst->states[0].final, fst->states[1].final, fst->states[2].final);
        // arcs
        struct _arc a = fst->states[0].arcs[0];
        printf("arc[0,0]: state=%d il=%d ol=%d w=%g\n", a.state, a.ilabel, a.olabel, a.weight);
        a = fst->states[0].arcs[1];
        printf("arc[0,1]: state=%d il=%d ol=%d w=%g\n", a.state, a.ilabel, a.olabel, a.weight);
        // relabel
        fst_relabel(fst, 1, 100, 0);  // input: 1->100
        a = fst->states[0].arcs[0];
        printf("after relabel input arc[0,0]: il=%d ol=%d\n", a.ilabel, a.olabel);
        // sort
        fst_arc_sort(fst, 1); // outer/output sort
        printf("flags after osort=%d\n", fst->flags);
        a = fst->states[0].arcs[0];
        printf("sorted arc[0,0]: il=%d ol=%d\n", a.ilabel, a.olabel);
        a = fst->states[0].arcs[1];
        printf("sorted arc[0,1]: il=%d ol=%d\n", a.ilabel, a.olabel);
        fst_remove(fst);
    } else if (strcmp(argv[1], "fst_io") == 0) {
        struct _fst * fst = fst_create();
        fst_add_state(fst);
        fst_add_state(fst);
        fst_add_state(fst);
        fst_add_arc(fst, 0, 1, 1, 2, 0.5f);
        fst_add_arc(fst, 1, 2, 3, 4, 1.0f);
        fst_set_final(fst, 2, 0.0f);
        fst_fwrite(fst, "/tmp/_test_fst.bin");
        fst_remove(fst);
        struct _fst * fst2 = fst_create();
        fst_fread(fst2, "/tmp/_test_fst.bin");
        printf("loaded n_states=%d sr_type=%d\n", fst2->n_states, fst2->sr_type);
        printf("loaded n_arcs(0)=%d n_arcs(1)=%d\n", fst2->states[0].n_arcs, fst2->states[1].n_arcs);
        struct _arc a = fst2->states[0].arcs[0];
        printf("arc: state=%d il=%d ol=%d w=%g\n", a.state, a.ilabel, a.olabel, a.weight);
        printf("final[2]=%d weight=%g\n", fst2->states[2].final, fst2->states[2].weight);
        fst_remove(fst2);
    } else if (strcmp(argv[1], "stack") == 0) {
        struct _fst * a = fst_create();
        fst_add_state(a); fst_add_state(a);
        fst_add_arc(a, 0, 1, 1, 1, 0.5f);
        fst_set_final(a, 1, 0.0f);
        struct _fst * b = fst_create();
        fst_add_state(b); fst_add_state(b);
        fst_add_arc(b, 0, 1, 2, 2, 1.0f);
        fst_set_final(b, 1, 0.0f);
        fst_stack(a, b);
        printf("stack n_states=%d\n", a->n_states);
        printf("state2 n_arcs=%d\n", a->states[2].n_arcs);
        struct _arc arc = a->states[2].arcs[0];
        printf("arc state=%d il=%d ol=%d\n", arc.state, arc.ilabel, arc.olabel);
        fst_remove(a);
        fst_remove(b);
    } else if (strcmp(argv[1], "iter") == 0) {
        struct _fst * fst = fst_create();
        fst_add_state(fst); fst_add_state(fst); fst_add_state(fst); fst_add_state(fst);
        fst_add_arc(fst, 0, 1, 1, 1, 0.0f);
        fst_add_arc(fst, 1, 2, 2, 2, 0.0f);
        fst_add_arc(fst, 2, 3, 3, 3, 0.0f);
        fst_set_final(fst, 3, 0.0f);
        struct _fst_iter * it = fst_iter_create(fst);
        state_t s;
        while ((s = fst_iter_next(it)) != (state_t)-1) {
            printf("iter: %d\n", s);
        }
        fst_iter_remove(it);
        fst_remove(fst);
    } else if (strcmp(argv[1], "reverse") == 0) {
        struct _fst * fst = fst_create();
        fst_add_state(fst); fst_add_state(fst); fst_add_state(fst);
        fst_add_arc(fst, 0, 1, 1, 1, 0.5f);
        fst_add_arc(fst, 1, 2, 2, 2, 1.0f);
        fst_set_final(fst, 2, 0.0f);
        fst_reverse(fst);
        printf("reverse n_states=%d start=%d\n", fst->n_states, fst->start);
        printf("final[0]=%d final[1]=%d final[2]=%d\n",
            fst->states[0].final, fst->states[1].final, fst->states[2].final);
        printf("n_arcs(0)=%d n_arcs(1)=%d n_arcs(2)=%d\n",
            fst->states[0].n_arcs, fst->states[1].n_arcs, fst->states[2].n_arcs);
        // Go through state 1 - should have arc to state 0
        struct _arc a = fst->states[1].arcs[0];
        printf("arc(1,0): state=%d il=%d ol=%d w=%g\n", a.state, a.ilabel, a.olabel, a.weight);
        a = fst->states[2].arcs[0];
        printf("arc(2,0): state=%d il=%d ol=%d w=%g\n", a.state, a.ilabel, a.olabel, a.weight);
        fst_remove(fst);
    } else if (strcmp(argv[1], "trim") == 0) {
        // build a graph that has unreachable and non-coaccessible states
        struct _fst * fst = fst_create();
        for (int i = 0; i < 5; ++i) fst_add_state(fst);
        fst_add_arc(fst, 0, 1, 1, 1, 1.0f);
        fst_add_arc(fst, 1, 2, 2, 2, 1.0f);
        fst_add_arc(fst, 0, 3, 3, 3, 1.0f); // 3 has no path to a final
        // 4 is unreachable
        fst_set_final(fst, 2, 0.0f);
        fst_trim(fst);
        printf("trim n_states=%d start=%d\n", fst->n_states, fst->start);
        for (state_t s = 0; s < fst->n_states; ++s) {
            printf("state %d final=%d n_arcs=%d\n", s, fst->states[s].final, fst->states[s].n_arcs);
        }
        fst_remove(fst);
    } else if (strcmp(argv[1], "rmstates") == 0) {
        struct _fst * fst = fst_create();
        for (int i = 0; i < 5; ++i) fst_add_state(fst);
        fst_add_arc(fst, 0, 1, 1, 1, 1.0f);
        fst_add_arc(fst, 1, 2, 2, 2, 1.0f);
        fst_add_arc(fst, 2, 4, 3, 3, 1.0f);
        fst_set_final(fst, 4, 0.0f);
        struct _bitset * mask = bitset_create(5);
        bitset_set(mask, 3); // remove state 3
        fst_rm_states(fst, mask);
        printf("rmstates n_states=%d\n", fst->n_states);
        for (state_t s = 0; s < fst->n_states; ++s) {
            printf("state %d final=%d n_arcs=%d\n", s, fst->states[s].final, fst->states[s].n_arcs);
            for (arc_t a = 0; a < fst->states[s].n_arcs; ++a) {
                struct _arc ar = fst->states[s].arcs[a];
                printf("  arc dst=%d il=%d ol=%d\n", ar.state, ar.ilabel, ar.olabel);
            }
        }
        bitset_remove(mask);
        fst_remove(fst);
    } else if (strcmp(argv[1], "shortest") == 0) {
        struct _fst * fst = fst_create();
        for (int i = 0; i < 5; ++i) fst_add_state(fst);
        fst_add_arc(fst, 0, 1, 1, 1, 1.0f);
        fst_add_arc(fst, 1, 2, 2, 2, 2.0f);
        fst_add_arc(fst, 0, 3, 3, 3, 0.5f);
        fst_add_arc(fst, 3, 2, 4, 4, 0.5f);
        fst_set_final(fst, 2, 0.0f);
        fst->start = 0;
        struct _fst * path = fst_create();
        fst_shortest(fst, path);
        printf("path n_states=%d\n", path->n_states);
        for (state_t s = 0; s < path->n_states; ++s) {
            printf("state %d final=%d n_arcs=%d\n", s, path->states[s].final, path->states[s].n_arcs);
            for (arc_t a = 0; a < path->states[s].n_arcs; ++a) {
                struct _arc ar = path->states[s].arcs[a];
                printf("  arc dst=%d il=%d ol=%d w=%g\n", ar.state, ar.ilabel, ar.olabel, ar.weight);
            }
        }
        fst_remove(fst);
        fst_remove(path);
    } else if (strcmp(argv[1], "compose") == 0) {
        // Two simple FSTs
        struct _fst * a = fst_create();
        for (int i = 0; i < 3; ++i) fst_add_state(a);
        fst_add_arc(a, 0, 1, 1, 2, 0.5f);
        fst_add_arc(a, 1, 2, 3, 4, 0.5f);
        fst_set_final(a, 2, 0.0f);
        struct _fst * b = fst_create();
        for (int i = 0; i < 3; ++i) fst_add_state(b);
        fst_add_arc(b, 0, 1, 2, 5, 0.5f);
        fst_add_arc(b, 1, 2, 4, 6, 0.5f);
        fst_set_final(b, 2, 0.0f);
        struct _fst * c = fst_create();
        fst_compose(a, b, c);
        printf("compose n_states=%d start=%d\n", c->n_states, c->start);
        for (state_t s = 0; s < c->n_states; ++s) {
            printf("state %d final=%d n_arcs=%d\n", s, c->states[s].final, c->states[s].n_arcs);
            for (arc_t aa = 0; aa < c->states[s].n_arcs; ++aa) {
                struct _arc ar = c->states[s].arcs[aa];
                printf("  arc dst=%d il=%d ol=%d w=%g\n", ar.state, ar.ilabel, ar.olabel, ar.weight);
            }
        }
        fst_remove(a); fst_remove(b); fst_remove(c);
    } else if (strcmp(argv[1], "match") == 0) {
        // unsorted match
        struct _arc aa[3] = {
            { .state = 1, .weight = 0.0f, .ilabel = 0, .olabel = 0 },
            { .state = 2, .weight = 0.0f, .ilabel = 1, .olabel = 5 },
            { .state = 3, .weight = 0.0f, .ilabel = 2, .olabel = 6 }
        };
        struct _arc bb[3] = {
            { .state = 4, .weight = 0.0f, .ilabel = 0, .olabel = 0 },
            { .state = 5, .weight = 0.0f, .ilabel = 5, .olabel = 7 },
            { .state = 6, .weight = 0.0f, .ilabel = 6, .olabel = 8 }
        };
        struct _queue * q = queue_create(sizeof(struct _match_item));
        match_unsorted(aa, bb, 3, 3, q);
        printf("unsorted matches=%zu\n", q->n_items);
        struct _match_item mi;
        while (queue_deque(q, &mi) != NULL) {
            printf("  a.ol=%d b.il=%d\n", mi.a.olabel, mi.b.ilabel);
        }
        queue_remove(q);
    } else if (strcmp(argv[1], "symt") == 0) {
        struct _symt * st = symt_create();
        symt_add(st, 1, "one");
        symt_add(st, 2, "two");
        symt_add(st, 3, "three");
        printf("get(1)=%s\n", symt_get(st, 1));
        printf("get(2)=%s\n", symt_get(st, 2));
        printf("get(3)=%s\n", symt_get(st, 3));
        printf("getr(one)=%zu\n", symt_getr(st, "one"));
        printf("getr(two)=%zu\n", symt_getr(st, "two"));
        printf("n_items=%zu\n", st->n_items);
        symt_remove(st);
    }
    return 0;
}
