cmake_minimum_required(VERSION 3.10)
cmake_path(GET CMAKE_CURRENT_SOURCE_DIR PARENT_PATH parent)
cmake_path(GET parent FILENAME project_name)
project(${project_name})
add_library(driver SHARED
    src/lib.c)

target_include_directories(driver PUBLIC include)
target_include_directories(driver PRIVATE src)

install(TARGETS driver 
    DESTINATION lib)
install(DIRECTORY include/
    DESTINATION include)


