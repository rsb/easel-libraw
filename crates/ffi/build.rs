// Build script for easel-libraw-ffi: compiles the vendored LibRaw C++ library
// into a static archive and generates Rust FFI bindings via bindgen.
//
// This is the slow side of the two-crate split. A full C++ compile takes ~35s,
// but cargo caches the result — downstream crates depending on easel-libraw get
// sub-second incremental builds as long as the vendored source hasn't changed.
//
// The build has three phases:
//   1. Compile all .cpp files under vendor/LibRaw-0.22.1/src/ into libraw.a,
//      with optional OpenMP support detected at build time.
//   2. Link the appropriate OpenMP runtime (libgomp for GNU, libomp for Clang)
//      if the probe succeeded.
//   3. Run bindgen against libraw.h with a tight allowlist to produce bindings.rs.
//      The allowlist keeps the FFI surface small — without it, bindgen would emit
//      hundreds of declarations for LibRaw's full internal API.

use std::env;
use std::path::{Path, PathBuf};

fn main() {
  let vendor = Path::new("vendor/LibRaw-0.22.1");
  let src_dir = vendor.join("src");

  let cpp_files = collect_cpp_files(&src_dir);
  let openmp = detect_openmp();

  // Phase 1: compile the C++ source tree into a static library.
  // warnings(false) suppresses LibRaw's internal warnings which we can't fix.
  let mut build = cc::Build::new();
  for file in &cpp_files {
    build.file(file);
  }

  build.cpp(true).include(vendor).warnings(false);

  if openmp {
    build.flag("-fopenmp");
    // macOS: Homebrew installs libomp headers outside the default search path.
    // Both /opt/homebrew (Apple Silicon) and /usr/local (Intel) are added
    // unconditionally since only the existing one will be used.
    if cfg!(target_os = "macos") {
      build.include("/opt/homebrew/include");
      build.include("/usr/local/include");
    }
  }

  build.compile("raw");

  // Phase 2: tell cargo to link the OpenMP runtime.
  // GNU toolchains use libgomp, Clang uses libomp. On macOS with Clang we also
  // need the Homebrew lib search path since libomp isn't in the default linker path.
  if openmp {
    let compiler = cc::Build::new().cpp(true).get_compiler();
    if compiler.is_like_gnu() {
      println!("cargo:rustc-link-lib=dylib=gomp");
    } else if compiler.is_like_clang() {
      println!("cargo:rustc-link-lib=dylib=omp");
      if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-search=/opt/homebrew/lib");
        println!("cargo:rustc-link-search=/usr/local/lib");
      }
    }
  }

  println!("cargo:rerun-if-changed=vendor/LibRaw-0.22.1");

  // Phase 3: generate Rust bindings from libraw.h.
  // The allowlist restricts output to the ~12 functions and ~5 types that the
  // safe wrapper crate actually uses. use_core() avoids a std dependency in the
  // generated code. generate_comments(false) drops C doc comments that don't
  // translate well to Rust. derive_default(true) adds Default impls for FFI
  // structs — not currently used but harmless.
  let header = vendor.join("libraw").join("libraw.h");
  let bindings = bindgen::Builder::default()
    .header(header.to_string_lossy())
    .clang_arg(format!("-I{}", vendor.display()))
    .allowlist_function("libraw_init")
    .allowlist_function("libraw_open_file")
    .allowlist_function("libraw_open_buffer")
    .allowlist_function("libraw_unpack")
    .allowlist_function("libraw_unpack_thumb")
    .allowlist_function("libraw_dcraw_process")
    .allowlist_function("libraw_dcraw_make_mem_image")
    .allowlist_function("libraw_dcraw_make_mem_thumb")
    .allowlist_function("libraw_dcraw_clear_mem")
    .allowlist_function("libraw_close")
    .allowlist_function("libraw_strerror")
    .allowlist_type("libraw_data_t")
    .allowlist_type("libraw_processed_image_t")
    .allowlist_type("LibRaw_image_formats")
    .allowlist_type("libraw_output_params_t")
    .allowlist_type("LibRaw_errors")
    .use_core()
    .generate_comments(false)
    .derive_default(true)
    .generate()
    .expect("failed to generate LibRaw bindings");

  let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
  bindings
    .write_to_file(out_path.join("bindings.rs"))
    .expect("failed to write LibRaw bindings");
}

// Writes a one-line C++ file that includes omp.h and calls omp_get_max_threads(),
// then tries to compile it with -fopenmp. If compilation succeeds, the system has
// a working OpenMP toolchain and we enable parallel demosaicing in LibRaw.
// On macOS the Homebrew include paths are added since libomp headers aren't in
// the default search path. Returns false (gracefully degrades) if OpenMP is
// unavailable — LibRaw falls back to single-threaded processing.
fn detect_openmp() -> bool {
  let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
  let probe_src = out_dir.join("openmp_probe.cpp");
  std::fs::write(&probe_src, "#include <omp.h>\nint main() { return omp_get_max_threads(); }\n")
    .expect("failed to write OpenMP probe source");

  let mut probe = cc::Build::new();
  probe.cpp(true).file(&probe_src).flag("-fopenmp");

  if cfg!(target_os = "macos") {
    probe.flag("-I/opt/homebrew/include");
    probe.flag("-I/usr/local/include");
  }

  probe.try_compile("openmp_probe").is_ok()
}

// Recursively collects all .cpp files under dir and returns them sorted.
// Sorting ensures deterministic compilation order across platforms — without it,
// readdir ordering varies by filesystem and can produce different object file
// layouts, defeating cargo's build cache.
fn collect_cpp_files(dir: &Path) -> Vec<PathBuf> {
  let mut files = Vec::new();
  collect_cpp_files_recursive(dir, &mut files);
  files.sort();
  files
}

// Walks dir recursively, appending any .cpp files to the accumulator.
// Silently skips directories that can't be read (e.g., broken symlinks in the
// vendor tree) rather than failing the build.
fn collect_cpp_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
  let entries = match std::fs::read_dir(dir) {
    Ok(e) => e,
    Err(_) => return,
  };
  for entry in entries.flatten() {
    let path = entry.path();
    if path.is_dir() {
      collect_cpp_files_recursive(&path, files);
    } else if path.extension().is_some_and(|e| e == "cpp") {
      files.push(path);
    }
  }
}
