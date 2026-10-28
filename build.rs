use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=assets/mellow.rc");
    println!("cargo:rerun-if-changed=assets/mellow-icon.ico");
    println!("cargo:rerun-if-env-changed=RC");
    println!("cargo:rerun-if-env-changed=WINDRES");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let msvc = env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    let resource = out.join(if msvc { "mellow.res" } else { "mellow.o" });
    let compiler = if msvc {
        env::var_os("RC").unwrap_or_else(|| "rc.exe".into())
    } else {
        env::var_os("WINDRES").unwrap_or_else(|| "windres".into())
    };
    let mut command = Command::new(&compiler);
    command.current_dir(&root);
    if msvc {
        command.args(["/nologo", "/I", "assets", "/fo"]);
        command.arg(&resource).arg("assets/mellow.rc");
    } else {
        command.args(["-I", "assets", "-i", "assets/mellow.rc", "-O", "coff", "-o"]);
        command.arg(&resource);
    }
    let status = command.status().unwrap_or_else(|error| {
        panic!("Cannot run {compiler:?} to embed the Mellow icon: {error}. Install Windows SDK (MSVC) or MinGW windres (GNU), or set RC/WINDRES.");
    });
    assert!(
        status.success(),
        "Compiling the Mellow icon failed: {status}"
    );
    println!("cargo:rustc-link-arg-bin=mellow={}", resource.display());
}
