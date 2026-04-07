cmake_minimum_required(VERSION 3.10)
project(translated_rust)
add_library(translated_rust SHARED
    ../c_src/src/lib.c)
target_include_directories(translated_rust PUBLIC ../c_src/include)
target_include_directories(translated_rust PRIVATE ../c_src/src)
