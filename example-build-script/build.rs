use conan2::{ConanInstall, ConanVerbosity};

fn main() {
    ConanInstall::new()
        .detect_profile() // Auto-detect "default" profile if not exists
        .build("missing")
        .verbosity(ConanVerbosity::Error) // Silence Conan warnings
        .run()
        .parse()
        .emit();
}
