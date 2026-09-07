set -u
OSSL_INC=/nix/store/dbvxz51s7m6401ycyp3l38407y11hq6p-openssl-3.6.3-dev/include
ROOT=$HARVEST_WORKDIR
OUT=$ROOT/.verify/cbuilds
for b in haraka sha2 shake blake; do for t in robust simple; do for s in 128s 128f 192s 192f 256s 256f; do
  d=$OUT/${b}_${t}_${s}
  mkdir -p $d
  (cd $d && cmake $ROOT/c_src -DCMAKE_POSITION_INDEPENDENT_CODE=ON -DHASH_BACKEND=$b -DSECPAR=$s -DTHASH=$t \
    -DCMAKE_C_FLAGS="-I$OSSL_INC" -DCMAKE_EXE_LINKER_FLAGS="-L$ROOT/.verify/osslib" -DCMAKE_SHARED_LINKER_FLAGS="-L$ROOT/.verify/osslib" > cmake.log 2>&1 \
    && cmake --build . -j4 > build.log 2>&1) && echo "BUILD OK $b $t $s" || echo "BUILD FAIL $b $t $s"
done; done; done
