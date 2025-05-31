use std::process::Command;

use conan2::{ConanInstall, ConanScope, ConanVerbosity};

fn main() {
    let status = Command::new("conan")
        .arg("create")
        .arg("test_pkg_conanfile.py")
        .status()
        .expect("failed to create test package");

    assert!(status.success(), "creating test package failed");

    ConanInstall::new()
        .host_profile("cargo-host")
        .build_profile("default")
        // Auto-detect "cargo-host" and "default" profiles if none exist
        .detect_profile()
        .build("missing")
        .verbosity(ConanVerbosity::Error) // Silence Conan warnings
        .option(ConanScope::Global, "shared", "False")
        .option(ConanScope::Local, "sanitizers", "True")
        .option(ConanScope::Package("openssl"), "no_deprecated", "True")
        .option(ConanScope::Package("libxml2/2.13.8"), "ftp", "False")
        .config("tools.build:skip_test", "True")
        .run()
        .parse()
        .emit();
}
