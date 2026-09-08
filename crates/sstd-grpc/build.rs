fn main() -> Result<(), Box<dyn std::error::Error>> {
    // No system protoc on this host: build protoc from vendored source and point
    // prost-build/tonic-build at it via the PROTOC env var.
    let protoc = protobuf_src::protoc();
    std::env::set_var("PROTOC", &protoc);

    // proto/ is the SINGLE source of truth for the wire schema. Compile every
    // *.proto in it automatically: adding a file here needs no Cargo.toml or
    // tonic-build wiring, and every consumer shares the one generated lib
    // (sstd-grpc) — never hand-roll wire structs or re-generate elsewhere.
    // See crates/sstd-grpc/proto/README.md (single-lib rule) + TDD_gRPC-Service-Index.
    let proto_dir = std::path::Path::new("proto");
    let mut protos: Vec<std::path::PathBuf> = std::fs::read_dir(proto_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().map_or(false, |ext| ext == "proto"))
        .collect();
    protos.sort();

    if protos.is_empty() {
        return Err("no .proto files found under proto/".into());
    }

    for path in &protos {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    println!("cargo:rerun-if-changed={}", proto_dir.display());

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&protos, &["proto"])?;
    Ok(())
}
