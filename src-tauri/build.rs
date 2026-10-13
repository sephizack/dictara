fn main() {
    println!("cargo:rerun-if-changed=qwen/Package.swift");
    println!("cargo:rerun-if-changed=qwen/Package.resolved");
    println!("cargo:rerun-if-changed=qwen/Sources");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos")
        && std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("aarch64")
    {
        build_qwen();
    }
    tauri_build::build()
}

fn build_qwen() {
    let manifest = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let scratch = manifest.parent().unwrap().join("target/qwen-swift");
    let mut command = std::process::Command::new("swift");
    command
        .arg("build")
        .arg("--package-path")
        .arg(manifest.join("qwen"))
        .arg("--scratch-path")
        .arg(&scratch)
        .args([
            "-c",
            "release",
            "--product",
            "dictara-qwen",
            "--disable-automatic-resolution",
        ]);
    assert!(
        command
            .status()
            .expect("Swift 6.2+ and Xcode are required for Qwen")
            .success(),
        "Qwen build failed; install Metal with xcodebuild -downloadComponent MetalToolchain"
    );
    let output = command.arg("--show-bin-path").output().unwrap();
    assert!(output.status.success(), "Cannot locate the Qwen build");
    let binary_dir = std::path::PathBuf::from(String::from_utf8(output.stdout).unwrap().trim());
    let destination = manifest.join("resources/qwen");
    std::fs::create_dir_all(&destination).unwrap();
    std::fs::copy(
        binary_dir.join("dictara-qwen"),
        destination.join("dictara-qwen"),
    )
    .unwrap();
    for entry in std::fs::read_dir(&binary_dir).unwrap() {
        let entry = entry.unwrap();
        if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "bundle")
        {
            copy_directory(&entry.path(), &destination.join(entry.file_name()));
        }
    }
    let result = std::process::Command::new("codesign")
        .args(["--force", "--sign", "-"])
        .arg(destination.join("dictara-qwen"))
        .status()
        .expect("Cannot sign the Qwen engine");
    assert!(result.success(), "Qwen engine signing failed");
}

fn copy_directory(source: &std::path::Path, destination: &std::path::Path) {
    std::fs::create_dir_all(destination).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_directory(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}
