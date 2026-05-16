# easel-libraw

Rust FFI bindings to LibRaw for RAW image decoding.

This crate provides safe Rust wrappers around LibRaw's C API, used by the Easel photo editor for importing camera RAW files. It is published as a standalone crate so downstream consumers do not need to build the C++ LibRaw source tree in their own workspace.
