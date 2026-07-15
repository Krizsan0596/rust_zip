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
