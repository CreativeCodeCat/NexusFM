cmake_minimum_required(VERSION 3.21)
project(nexusfm-qt LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

find_package(Qt6 6.5 REQUIRED COMPONENTS Widgets)

# The Rust core is built by Cargo from the workspace root and linked statically.
set(NFM_RUST_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/..")
if(CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(NFM_RUST_PROFILE debug)
    set(NFM_CARGO_FLAGS "")
else()
    set(NFM_RUST_PROFILE release)
    set(NFM_CARGO_FLAGS --release)
endif()
# NFM_RUST_LIB can point at an already-built libnexusfm_ffi.a (the cargo build
# script does this). When it is empty, CMake builds the library itself.
set(NFM_RUST_LIB "" CACHE FILEPATH "Prebuilt libnexusfm_ffi.a; empty to build it here")
if(NFM_RUST_LIB)
    add_custom_target(nexusfm_rust_core)
else()
    set(NFM_RUST_LIB "${NFM_RUST_ROOT}/target/${NFM_RUST_PROFILE}/libnexusfm_ffi.a")
    add_custom_target(nexusfm_rust_core
        COMMAND ${CMAKE_COMMAND} -E env cargo build -p nexusfm-ffi ${NFM_CARGO_FLAGS}
        WORKING_DIRECTORY "${NFM_RUST_ROOT}"
        BYPRODUCTS "${NFM_RUST_LIB}"
        COMMENT "Building Rust core (nexusfm-ffi)"
        VERBATIM)
endif()

qt_add_resources(NFM_RESOURCES resources.qrc)

add_executable(nexusfm-qt
    ${NFM_RESOURCES}
    src/main.cpp
    src/Core.h
    src/Core.cpp
    src/FileModel.h
    src/FileModel.cpp
    src/MainWindow.h
    src/MainWindow.cpp
    src/TitleBar.h
    src/TitleBar.cpp
    src/nexusfm_ffi.h)
add_dependencies(nexusfm-qt nexusfm_rust_core)

target_include_directories(nexusfm-qt PRIVATE src)
target_link_libraries(nexusfm-qt PRIVATE
    Qt6::Widgets
    "${NFM_RUST_LIB}"
    pthread dl m gcc_s util rt)
