cmake_minimum_required(VERSION 3.10)
project(driver_shared)
add_library(driver SHARED ../c_src/src/main.c ../c_src/src/inventory.c)
target_include_directories(driver PUBLIC ../c_src/include)
set_target_properties(driver PROPERTIES POSITION_INDEPENDENT_CODE ON)
# Also build the executable for binary comparison
add_executable(driver_bin ../c_src/src/main.c ../c_src/src/inventory.c)
target_include_directories(driver_bin PUBLIC ../c_src/include)
set_target_properties(driver_bin PROPERTIES OUTPUT_NAME "driver")
