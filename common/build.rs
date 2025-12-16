fn main() -> std::io::Result<()> {
    // The output directory is *relative* to the project root
    let out_dir = std::path::PathBuf::from("src/proto");

    prost_build::Config::new()
        .out_dir(out_dir) // Directs output to src/proto/
        .compile_protos(
            &["proto/space.proto"], // List of all .proto files
            &["proto"],                 // The root directory for .proto files
        )?;
    Ok(())
}
