cmake_minimum_required(VERSION 3.10)
project(shadow_c C)

# Probe build of the ground-truth C library. lib_shadow.c #includes the
# untouched ../c_src/src/lib.c, so c_src/ is never modified.
add_library(shadow_c SHARED lib_shadow.c)

target_include_directories(shadow_c PRIVATE ../c_src/include ../c_src/src)

# c_src/CMakeLists.txt does not link m; do it here since the probe build
# actually reaches the sqrt() call in mode 0004.
find_library(M_LIB m)
if(M_LIB)
    target_link_libraries(shadow_c PRIVATE ${M_LIB})
endif()

# The included lib.c intentionally contains a never-called static function.
target_compile_options(shadow_c PRIVATE -Wno-unused-function)
