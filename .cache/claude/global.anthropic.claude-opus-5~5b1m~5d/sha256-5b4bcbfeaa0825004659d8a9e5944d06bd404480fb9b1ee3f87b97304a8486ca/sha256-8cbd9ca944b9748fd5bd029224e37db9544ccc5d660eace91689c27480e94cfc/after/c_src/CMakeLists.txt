# Copyright 2025 MIT Lincoln Laboratory
# Permission is hereby granted, free of charge,
# to any person obtaining a copy of this software
# and associated documentation files (the “Software”),
# to deal in the Software without restriction,
# including without limitation the rights to use, copy,
# modify, merge, publish, distribute, sublicense,
# and/or sell copies of the Software,
# and to permit persons to whom the Software is furnished to do so,
# subject to the following conditions:
#
# The above copyright notice and this permission notice
# shall be included in all copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
# EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
# THE WARRANTIES OF MERCHANTABILITY,
# FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
# IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
# FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
# TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
# OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

cmake_minimum_required(VERSION 3.10)
cmake_path(GET CMAKE_CURRENT_SOURCE_DIR PARENT_PATH parent)
cmake_path(GET parent FILENAME project_name)
project(${project_name})

add_library(${project_name} SHARED src/match.c src/spectral_contrast.c)

target_include_directories(${project_name} PUBLIC include)
target_include_directories(${project_name} PRIVATE src)

target_link_libraries(${project_name} m)

install(TARGETS ${project_name}
        DESTINATION lib)
install(DIRECTORY include/
        DESTINATION include)
