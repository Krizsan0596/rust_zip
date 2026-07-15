use std::process::Command;
use std::fs;

#[test]
fn test_cli_decompression_panic() {
    let exe = env!("CARGO_BIN_EXE_rust_zip");
    let input_file = tempfile::NamedTempFile::new().unwrap();
    let output_file = tempfile::NamedTempFile::new().unwrap();
    let input_path = input_file.path();
    let output_path = output_file.path();

    // Create an input file for decompression
    fs::write(input_path, b"some compressed data").unwrap();

    let output = Command::new(exe)
        .arg("-x")
        .arg("-o")
        .arg(output_path)
        .arg(input_path)
        .output()
        .expect("failed to execute process");

    // The decompression path must fail (panic) because of the empty Tree unwrapping root.
    assert!(!output.status.success(), "Process succeeded but was expected to fail due to panic");
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("panicked") || stderr.contains("unwrap"),
        "Expected panic output in stderr, got: {}", stderr
    );
}

#[test]
fn test_cli_compression_success() {
    let exe = env!("CARGO_BIN_EXE_rust_zip");
    let input_file = tempfile::NamedTempFile::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let output_path = dir.path().join("output.tmp");
    let input_path = input_file.path();

    // Create a tiny input file with multiple symbols so it's a valid tree
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

    // Verify output file was created and is not empty
    let metadata = fs::metadata(&output_path).expect("Output file was not created");
    assert!(metadata.len() > 0, "Output file is empty");
}
