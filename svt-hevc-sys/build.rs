use anyhow::Context;
use std::env;
use std::path::{Path, PathBuf};

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=svt-hevc.h");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let source_path = manifest_dir.join("SVT-HEVC");
    let out_path = PathBuf::from(env::var("OUT_DIR")?);

    // Patch the logging macro to call our rust fn.
    let patched_header = out_path.join("EbDefinitions_PATCHED.h");
    apply_patch(
        "SVT-HEVC/Source/Lib/Codec/EbDefinitions.h",
        &patched_header,
        manifest_dir.join("logging.patch"),
    )
    .context("failed to apply logging patch")?;

    // Build the library.
    let compile_path = cmake::Config::new(&source_path)
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("BUILD_APP", "OFF")
        .profile("Release") // The encoder does an awful lot of printf() in debug mode.
        // This injects our header file during compilation. The patched header
        // file requires EbApi.h, so we have to add that include path as well.
        .cflag(format!("-I{}/Source/API", source_path.display()))
        .cflag(format!("-include{}", patched_header.display()))
        .build();

    println!(
        "cargo:rustc-link-search=native={}/lib",
        compile_path.display()
    );
    println!("cargo:rustc-link-lib=static=SvtHevcEnc");
    println!("cargo:rustc-link-lib=pthread");

    // Generate bindings.
    let bindings = bindgen::Builder::default()
        .clang_args([format!("-I{}/include/svt-hevc", compile_path.display())])
        .header("svt-hevc.h")
        .allowlist_item("E[Bb].*")
        .derive_default(true)
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
