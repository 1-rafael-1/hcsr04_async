use std::env;
use std::process::Command;

#[test]
fn compile_examples() {
    // Get the current directory
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let examples_dir = current_dir.join("examples");

    println!("Building examples in {:?}", examples_dir);

    // Run cargo build in the examples directory with the correct target
    let output = Command::new("cargo")
        .current_dir(&examples_dir)
        .args(["build", "--target", "thumbv6m-none-eabi", "-vv"])
        .output()
        .expect("Failed to execute cargo build");

    // Print the output regardless of success/failure
    println!("Build stdout:\n{}", String::from_utf8_lossy(&output.stdout));
    println!("Build stderr:\n{}", String::from_utf8_lossy(&output.stderr));

    // Check if the build was successful
    assert!(output.status.success(), "Failed to compile examples");
}
