use anyhow::Context;
use std::env;
use std::path::{Path, PathBuf};

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=svt-av1.h");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let source_path = manifest_dir.join("SVT-AV1");
    let out_path = PathBuf::from(env::var("OUT_DIR")?);

    let mut cmake_build = cmake::Config::new(source_path);
    cmake_build
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("BUILD_APPS", "OFF")
        .define("BUILD_DEC", "OFF")
        // The encoder does an awful lot of printf() in debug mode.
        .profile("Release");

    if cfg!(feature = "log") {
        // Patch the logging macro to call our rust fn.
        let patched_header = out_path.join("svt_log_PATCHED.h");

        apply_patch(
            "SVT-AV1/Source/Lib/Codec/svt_log.h",
            &patched_header,
            manifest_dir.join("logging.patch"),
        )
        .context("failed to apply logging patch")?;

        // Insert the header.
        cmake_build.cflag(format!("-include{}", patched_header.display()));
    } else {
        // Disable logging.
        cmake_build.define("SVT_LOG_QUIET", "1");
    }

    // Build the library.
    let compile_path = cmake_build.build();

    println!(
        "cargo:rustc-link-search=native={}/lib",
        compile_path.display()
    );

    println!("cargo:rustc-link-lib=static=SvtAv1Enc");
    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rustc-link-lib=m");

    // Generate bindings.
    let bindings = bindgen::Builder::default()
        .clang_args([format!("-I{}/include/svt-av1", compile_path.display())])
        .header("svt-av1.h")
        .allowlist_item("E[Bb].*")
        .allowlist_item("svt_av1_.*")
        .allowlist_item("Svt.*")
        .derive_default(true)
        .generate_comments(false)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .context("failed to generate bindings")?;

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .context("failed to generate bindings")?;

    Ok(())
}

fn apply_patch(
    in_file: impl AsRef<Path>,
    out_file: impl AsRef<Path>,
    patch_file: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let src = std::fs::read_to_string(in_file).context("failed to read input file")?;
    let mut dst =
        std::fs::File::create(out_file.as_ref()).context("failed to create patched file")?;

    let patch = std::fs::read_to_string(patch_file.as_ref())?;
    let patch = diffy::Patch::from_str(&patch)?;

    let patched = diffy::apply(&src, &patch)?;
    std::io::Write::write_all(&mut dst, patched.as_bytes())?;
    Ok(())
}
