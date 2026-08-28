# .pristine/

Byte-exact snapshots of `../src/*.rs`, used by `../mutation_check.py` to restore
the sources after each injected mutant (including from a `finally:` block, so an
interrupted run cannot leave the tree modified).

These files are **not** compiled — `Cargo.toml` builds only `src/lib.rs`. If you
change anything under `src/`, refresh the snapshot:

    cp src/*.rs .pristine/
