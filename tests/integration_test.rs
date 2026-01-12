//! conan2-rs integration tests

use std::{io::Write, path::Path};

use conan2::{ConanInstall, ConanScope, ConanVerbosity};

#[test]
fn run_conan_install() {
    let output = ConanInstall::with_recipe(Path::new("tests/conanfile.txt"))
        .output_folder(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .detect_profile() // Auto-detect "default" profile if not exists
        .build_type("Release")
        .build("missing")
        .verbosity(ConanVerbosity::Verbose)
        .option(ConanScope::Global, "shared", "True")
        .option(ConanScope::Local, "sanitizers", "True")
        .option(ConanScope::Package("openssl"), "no_deprecated", "True")
        .option(ConanScope::Package("libxml2/2.15.0"), "programs", "False")
        .config("tools.build:skip_test", "True")
        .remote("conancenter") // The default Conan remote, just to test `--remote`
        .arg("--core-conf")
        .arg("core:non_interactive=True")
        .run();

    // Fallback for test debugging
    if !output.is_success() {
        std::io::stderr().write_all(output.stderr()).unwrap();
    }

    assert!(output.is_success());
    assert_eq!(output.status_code(), 0);

    let cargo = output.parse();
    let includes = cargo.include_paths();

    assert!(includes.len() > 3);

    cargo.emit();
}

#[test]
fn fail_no_conanfile() {
    let output = ConanInstall::new()
        .output_folder(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .build_type("Debug")
        .verbosity(ConanVerbosity::Status)
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
fn fail_no_remote() {
    let output = ConanInstall::with_recipe(Path::new("tests/conanfile.txt"))
        .output_folder(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .remote("no-such-remote")
        .verbosity(ConanVerbosity::Debug)
        .run();

    std::io::stderr().write_all(output.stderr()).unwrap();

    assert!(!output.is_success());
    assert_eq!(output.status_code(), 1);
    assert_eq!(output.stdout().len(), 0);
    assert!(output.stderr().starts_with(b"ERROR: Remote '"));
}

#[test]
fn fail_no_profile() {
    let output = ConanInstall::with_recipe(Path::new("tests/conanfile.txt"))
        .output_folder(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .profile("no-such-profile")
        .verbosity(ConanVerbosity::Debug)
        .run();

    std::io::stderr().write_all(output.stderr()).unwrap();

    assert!(!output.is_success());
    assert_eq!(output.status_code(), 1);
    assert_eq!(output.stdout().len(), 0);
    assert!(output.stderr().starts_with(b"ERROR: Profile not found: "));
}

#[test]
fn fail_unknown_args() {
    let output = ConanInstall::with_recipe(Path::new("tests/conanfile.txt"))
        .output_folder(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .arg("--no-such-argument")
        .verbosity(ConanVerbosity::Debug)
        .run();

    std::io::stderr().write_all(output.stderr()).unwrap();

    assert!(!output.is_success());
    assert_eq!(output.status_code(), 2);
    assert_eq!(output.stdout().len(), 0);
    assert!(!output.stderr().is_empty());
    assert!(str::from_utf8(output.stderr())
        .unwrap()
        .contains("error: unrecognized arguments: --no-such-argument"));
}

#[test]
fn detect_custom_profile() {
    let output = ConanInstall::with_recipe(Path::new("tests/conanfile.txt"))
        .output_folder(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .profile(&format!("{}-dynamic-profile", env!("CARGO_PKG_NAME")))
        .detect_profile()
        .build_type("RelWithDebInfo")
        .build("missing")
        .verbosity(ConanVerbosity::Debug)
        .run();

    std::io::stderr().write_all(output.stderr()).unwrap();
    assert!(output.is_success());
}

#[test]
fn host_and_build_profiles() {
    let output = ConanInstall::with_recipe(Path::new("tests/conanfile.txt"))
        .output_folder(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .host_profile(&format!("{}-dynamic-host-profile", env!("CARGO_PKG_NAME")))
        .build_profile(&format!("{}-dynamic-build-profile", env!("CARGO_PKG_NAME")))
        .detect_profile()
        .build("missing")
        .verbosity(ConanVerbosity::Debug)
        .run();

    std::io::stderr().write_all(output.stderr()).unwrap();
    assert!(output.is_success());
}

#[test]
fn test_shared_and_exe_link_flags() {
    let output = ConanInstall::with_recipe(Path::new("tests/conanfile.txt"))
        .option(ConanScope::Package("soxr"), "with_openmp", "True")
        .output_folder(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .detect_profile()
        .build("missing")
        .verbosity(ConanVerbosity::Debug)
        .run();

    assert!(output.is_success());
    let cargo = output.parse();
    let emitted_instructions = String::from_utf8(cargo.as_bytes().to_vec()).expect("Invalid UTF-8");
    assert!(emitted_instructions.contains("cargo:rustc-cdylib-link-arg=-fopenmp"));
    assert!(emitted_instructions.contains("cargo:rustc-link-arg-bins=-fopenmp"));
}
