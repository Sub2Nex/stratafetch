use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=app.manifest");
    println!("cargo:rerun-if-changed=app.rc");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-bins=/MANIFESTINPUT:{}",
            root.join("app.manifest").display()
        );
    } else {
        let target = env::var("TARGET").unwrap();
        let compiler_var = format!("RC_{}", target.replace('-', "_"));
        println!("cargo:rerun-if-env-changed={compiler_var}");
        println!("cargo:rerun-if-env-changed=RC");
        let default_compiler = match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
            Ok("x86_64") => "x86_64-w64-mingw32-windres",
            Ok("x86") => "i686-w64-mingw32-windres",
            _ => panic!("Set {compiler_var} to a target-compatible resource compiler"),
        };
        let compiler = env::var(&compiler_var)
            .or_else(|_| env::var("RC"))
            .unwrap_or_else(|_| default_compiler.into());
        let resource = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("app-manifest.o");
        let status = Command::new(&compiler)
            .current_dir(&root)
            .arg("--input")
            .arg("app.rc")
            .arg("--output-format")
            .arg("coff")
            .arg("--output")
            .arg(&resource)
            .status()
            .unwrap_or_else(|error| panic!("Cannot run resource compiler {compiler}: {error}"));
        assert!(
            status.success(),
            "Windows application manifest compilation failed"
        );
        println!("cargo:rustc-link-arg-bins={}", resource.display());
    }
}
