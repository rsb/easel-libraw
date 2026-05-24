# rsb-libraw

Safe Rust bindings to [LibRaw](https://www.libraw.org/) for camera RAW
image decoding. Supports approximately 1,500 camera models (CR2, CR3,
NEF, ARW, RAF, RW2, DNG, and more).

This repository exists so downstream consumers get pre-built LibRaw
bindings without compiling 60k lines of C++ in their own workspace.

## Architecture

Two crates, one boundary:

```
┌─────────────────────────────────────────────────────┐
│  rsb-libraw              (pure Rust, fast build)   │
│                                                     │
│  pub mod error   — Kind, Error, ResultExt           │
│  pub mod image   — Pixel, ImageBuffer               │
│  pub mod decode  — RawDecode trait                  │
│  mod adapter     — LibRawProcessor, LibRawAdapter   │
└─────────────────────┬───────────────────────────────┘
                      │ depends on
┌─────────────────────▼───────────────────────────────┐
│  rsb-libraw-ffi          (C++ build, slow, cached) │
│                                                     │
│  build.rs        — cc compiles vendor C++ source    │
│                  — bindgen generates FFI bindings    │
│  src/lib.rs      — re-exports generated bindings    │
│  vendor/         — LibRaw 0.22.1 source tree        │
└─────────────────────────────────────────────────────┘
```

**Why two crates?** The FFI crate compiles C++ and runs bindgen — this
takes ~35s on first build. Once cached, it never rebuilds unless the
vendored source changes. The pure Rust crate compiles in under a
second, so iterating on the adapter logic is fast.

## Build prerequisites

A C++ compiler and `libclang` are required for the FFI crate:

- **Linux:** `sudo apt install g++ libclang-dev libgomp1`
- **macOS:** `xcode-select --install` && `brew install llvm libomp`

OpenMP is optional. When available, LibRaw's demosaic loops run across
all cores. Without it, decoding is single-threaded but still correct.

## Usage examples

### Full-resolution decode

```rust
use std::path::Path;
use rsb_libraw::{LibRawAdapter, RawDecode};

let adapter = LibRawAdapter;
let path = Path::new("/photos/DSC_0042.NEF");

let image = adapter.decode(path)?;
println!("{}x{}, {} pixels", image.width(), image.height(), image.pixels().len());
```

### Three-phase progressive decode

The adapter supports three decode methods for progressive display.
Call them in order — each produces a better result than the last:

```rust
use std::path::Path;
use rsb_libraw::{LibRawAdapter, RawDecode, ImageBuffer};

let adapter = LibRawAdapter;
let path = Path::new("/photos/IMG_4521.CR3");

// Phase 1: embedded JPEG thumbnail (~120ms)
// Almost every camera embeds a full-res JPEG in the RAW container.
let thumbnail: ImageBuffer = adapter.decode_thumbnail(path)?;
display(&thumbnail);

// Phase 2: half-size demosaic (~260ms with OpenMP)
// Quarter pixel count, proper color processing.
let preview: ImageBuffer = adapter.decode_preview(path)?;
display(&preview);

// Phase 3: full-resolution 16-bit decode (~3s with OpenMP)
// Camera white balance, sRGB output, full pixel count.
let full: ImageBuffer = adapter.decode(path)?;
display(&full);
```

### Error handling

Errors are classified by recovery action:

```rust
use rsb_libraw::{LibRawAdapter, RawDecode};
use rsb_libraw::error::Kind;

let adapter = LibRawAdapter;

match adapter.decode(path) {
  Ok(image) => process(image),
  Err(e) => match e.kind() {
    Kind::Io          => eprintln!("file not found or bad path: {e}"),
    Kind::Unsupported => eprintln!("camera/format not handled: {e}"),
    Kind::Corrupt     => eprintln!("file data is malformed: {e}"),
    Kind::Resource    => eprintln!("out of memory: {e}"),
  }
}
```

### Reading pixel data

`ImageBuffer` stores pixels as packed sRGB `u32` values:

```rust
use rsb_libraw::image::Pixel;

let image = adapter.decode(path)?;

for pixel in image.pixels() {
  let r = pixel.r(); // u8, 0-255
  let g = pixel.g();
  let b = pixel.b();
  let packed = pixel.packed(); // 0x00RRGGBB
}

// Coordinate-based access with bounds checking
if let Some(pixel) = image.get(100, 200) {
  println!("r={} g={} b={}", pixel.r(), pixel.g(), pixel.b());
}

// Or direct slice access for bulk operations (row-major layout)
let pixel = image.pixels()[(y * image.width() + x) as usize];
```

### Using the trait for abstraction

The `RawDecode` trait allows swapping implementations without
changing calling code:

```rust
use std::path::Path;
use rsb_libraw::{RawDecode, LibRawAdapter, ImageBuffer, Error};

fn import_raw(decoder: &dyn RawDecode, path: &Path) -> Result<ImageBuffer, Error> {
  decoder.decode(path)
}

// Production: use LibRaw
let decoder: Box<dyn RawDecode> = Box::new(LibRawAdapter);
let image = import_raw(&*decoder, Path::new("photo.arw"))?;
```

## Error classification

| Kind | Meaning | Caller action |
|------|---------|---------------|
| `Io` | Path invalid or file not accessible | Check path, permissions |
| `Unsupported` | Input recognized but format not handled | Skip file or upgrade |
| `Corrupt` | Data present but malformed | Reject file, report to user |
| `Resource` | System resource exhaustion (OOM) | Back off or abort |

## Vendored LibRaw source

LibRaw 0.22.1 is vendored under `crates/ffi/vendor/LibRaw-0.22.1/`.

- Source: https://github.com/LibRaw/LibRaw/releases/tag/0.22.1
- License: CDDL-1.0 (file-level copyleft). Only modifications to
  LibRaw source files trigger disclosure. This repo does not modify
  vendored source.

## Updating LibRaw

1. Download the new release from GitHub
2. Replace `crates/ffi/vendor/LibRaw-0.22.1/` with the new version
3. Update the path in `crates/ffi/build.rs`
4. Run `cargo check` to verify bindings generate correctly
5. Run the full test suite against sample RAW files
6. Update this README with the new version number
