fn main() -> Result<(), Box<dyn std::error::Error>> {
    // No system protoc on this host: build protoc from vendored source and point
    // prost-build/tonic-build at it via the PROTOC env var.
    let protoc = protobuf_src::protoc();
    std::env::set_var("PROTOC", &protoc);
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["proto/editor.proto"], &["proto"])?;
    Ok(())
}
