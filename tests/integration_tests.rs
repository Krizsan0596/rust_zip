use std::fs;
use std::process::Command;

#[test]
fn test_cli_decompression_panic() {
    let exe = env!("CARGO_BIN_EXE_rust_zip");
    let input_file = tempfile::NamedTempFile::new().unwrap();
    let output_dir = tempfile::tempdir().unwrap();
    let input_path = input_file.path();
    let output_path = output_dir.path().join("output.tmp");

    fs::write(input_path, b"some compressed data").unwrap();

    let output = Command::new(exe)
        .arg("-x")
        .arg("-o")
        .arg(&output_path)
        .arg(input_path)
        .output()
        .expect("failed to execute process");

    assert!(
        !output.status.success(),
        "Decompression unexpectedly succeeded: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        !output.stderr.is_empty(),
        "Expected error output on stderr when decompression fails"
    );
}

#[test]
fn test_cli_compression_success() {
    let exe = env!("CARGO_BIN_EXE_rust_zip");
    let input_file = tempfile::NamedTempFile::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let output_path = dir.path().join("output.tmp");
    let input_path = input_file.path();

    fs::write(input_path, b"hello world").unwrap();

    let output = Command::new(exe)
        .arg("-c")
        .arg("-o")
        .arg(&output_path)
        .arg(input_path)
        .output()
        .expect("failed to execute process");

    assert!(
        output.status.success(),
        "Process failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let metadata = fs::metadata(&output_path).expect("Output file was not created");
    assert!(metadata.len() > 0, "Output file is empty");
}

#[test]
fn test_cli_round_trip() {
    let exe = env!("CARGO_BIN_EXE_rust_zip");

    let dir = tempfile::tempdir().unwrap();
    let input_path = dir.path().join("input.txt");
    let compressed_path = dir.path().join("compressed.bin");
    let decompressed_path = dir.path().join("decompressed.txt");

    let input_content = "Lorem ipsum dolor sit amet consectetur adipiscing elit. Quisque faucibus ex sapien vitae pellentesque sem placerat. In id cursus mi pretium tellus duis convallis. Tempus leo eu aenean sed diam urna tempor. Pulvinar vivamus fringilla lacus nec metus bibendum egestas. Iaculis massa nisl malesuada lacinia integer nunc posuere. Ut hendrerit semper vel class aptent taciti sociosqu. Ad litora torquent per conubia nostra inceptos himenaeos.";

    fs::write(&input_path, input_content).unwrap();

    let compress_output = Command::new(exe)
        .arg("-c")
        .arg("-o")
        .arg(&compressed_path)
        .arg(&input_path)
        .output()
        .expect("failed to execute compression");

    assert!(
        compress_output.status.success(),
        "Compression failed: {}",
        String::from_utf8_lossy(&compress_output.stderr)
    );

    let compressed_metadata =
        fs::metadata(&compressed_path).expect("Compressed file was not created");
    assert!(compressed_metadata.len() > 0, "Compressed file is empty");

    let decompress_output = Command::new(exe)
        .arg("-x")
        .arg("-o")
        .arg(&decompressed_path)
        .arg(&compressed_path)
        .output()
        .expect("failed to execute decompression");

    assert!(
        decompress_output.status.success(),
        "Decompression failed: {}",
        String::from_utf8_lossy(&decompress_output.stderr)
    );

    let output_content =
        fs::read_to_string(&decompressed_path).expect("Failed to read decompressed file");

    pretty_assertions::assert_eq!(input_content, output_content);
}
