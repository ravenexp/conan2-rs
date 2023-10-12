//! conan2-rs integration tests

use std::{io::Write, path::Path};

use conan2::ConanInstall;

#[test]
fn run_conan_install() {
    let output = ConanInstall::with_recipe(Path::new("tests/conanfile.txt"))
        .output_folder(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .build("missing")
        .run();

    // Fallback for test debugging
    if !output.is_success() {
        std::io::stderr().write_all(output.stderr()).unwrap();
    }

    assert!(output.is_success());
    assert_eq!(output.status_code(), 0);

    output.parse().emit();
}

#[test]
fn fail_no_conanfile() {
    let output = ConanInstall::new()
        .output_folder(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .run();

    std::io::stderr().write_all(output.stderr()).unwrap();

    assert!(!output.is_success());
    assert_eq!(output.status_code(), 1);
    assert_eq!(output.stdout().len(), 0);
    assert!(output
        .stderr()
        .starts_with(b"ERROR: Conanfile not found at"));
}

#[test]
fn fail_no_profile() {
    let output = ConanInstall::with_recipe(Path::new("tests/conanfile.txt"))
        .output_folder(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .profile("no-such-profile")
        .run();

    std::io::stderr().write_all(output.stderr()).unwrap();

    assert!(!output.is_success());
    assert_eq!(output.status_code(), 1);
    assert_eq!(output.stdout().len(), 0);
    assert!(output.stderr().starts_with(b"ERROR: Profile not found: "));
}
