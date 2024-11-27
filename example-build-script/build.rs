use conan2::{ConanInstall, ConanVerbosity};

fn main() {
    ConanInstall::new()
        .host_profile("cargo-host")
        .build_profile("default")
        // Auto-detect "cargo-host" and "default" profiles if none exist
        .detect_profile()
        .build("missing")
        .verbosity(ConanVerbosity::Error) // Silence Conan warnings
        .run()
        .parse()
        .emit();
}
