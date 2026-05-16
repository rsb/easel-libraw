use std::env;
use std::path::{Path, PathBuf};

fn main() {
  let vendor = Path::new("vendor/LibRaw-0.22.1");
  let src_dir = vendor.join("src");

  let cpp_files = collect_cpp_files(&src_dir);
  let openmp = detect_openmp();

  let mut build = cc::Build::new();
  for file in &cpp_files {
    build.file(file);
  }

  build.cpp(true).include(vendor).warnings(false);

  if openmp {
    build.flag("-fopenmp");
    if cfg!(target_os = "macos") {
      build.include("/opt/homebrew/include");
      build.include("/usr/local/include");
    }
  }

  build.compile("raw");

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

fn collect_cpp_files(dir: &Path) -> Vec<PathBuf> {
  let mut files = Vec::new();
  collect_cpp_files_recursive(dir, &mut files);
  files.sort();
  files
}

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
