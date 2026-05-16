# easel-libraw

Rust FFI bindings to LibRaw for RAW image decoding.

This crate provides safe Rust wrappers around LibRaw's C API, used by the Easel photo editor for importing camera RAW files. It is published as a standalone crate so downstream consumers do not need to build the C++ LibRaw source tree in their own workspace.

## Workspace structure

```
crates/
  fail/          lab-fail        — error vocabulary
  image/         lab-image       — Pixel + ImageBuffer types
  raw-decoder/   lab-raw-decoder — RawDecode trait
  libraw/        lab-libraw      — LibRaw FFI adapter (vendored C++ build)
```

## Build prerequisites

A C++ compiler and `libclang` are required:

- **Linux:** `sudo apt install g++ libclang-dev`
- **macOS:** `xcode-select --install` + `brew install llvm`

## Usage

```rust
use lab_libraw::LibRawAdapter;
use lab_raw_decoder::RawDecode;

let adapter = LibRawAdapter;
let image = adapter.decode(path)?;
```
