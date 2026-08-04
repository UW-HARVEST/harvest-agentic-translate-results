cmake_minimum_required(VERSION 3.10)
project(driver)

add_executable(driver src/q_math.c src/main.c)

target_link_libraries(driver m)

target_include_directories(driver PUBLIC inc)
target_include_directories(driver PRIVATE src)
