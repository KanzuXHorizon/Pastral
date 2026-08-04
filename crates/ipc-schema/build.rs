use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo must set CARGO_MANIFEST_DIR"),
    );
    let schema_root = manifest_dir.join("../../protocols/ipc-schema");
    let schema_name = Path::new("pastral_ipc_v1.proto");
    let schema = schema_root.join(schema_name);
    let protoc = resolve_protoc();

    println!("cargo:rerun-if-changed={}", schema.display());
    println!("cargo:rerun-if-env-changed=PROTOC");
    protobuf_codegen::CodeGen::new()
        .protoc_path(protoc)
        .inputs([schema_name])
        .include(schema_root)
        .generate_and_compile()
        .expect("exact Protocol Buffers Rust code generation must succeed");
}

fn resolve_protoc() -> PathBuf {
    if let Some(path) = std::env::var_os("PROTOC") {
        return PathBuf::from(path);
    }
    if let Some(path) = find_winget_protoc() {
        return path;
    }
    PathBuf::from("protoc")
}

fn find_winget_protoc() -> Option<PathBuf> {
    let local_app_data = PathBuf::from(std::env::var_os("LOCALAPPDATA")?);
    let packages = local_app_data.join("Microsoft/WinGet/Packages");
    let entries = std::fs::read_dir(packages).ok()?;
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            winget_package_name(path).is_some_and(|name| name.starts_with("Google.Protobuf_"))
        })
        .map(|path| path.join("bin/protoc.exe"))
        .find(|candidate| candidate.is_file())
}

fn winget_package_name(path: &Path) -> Option<&str> {
    path.file_name()?.to_str()
}
