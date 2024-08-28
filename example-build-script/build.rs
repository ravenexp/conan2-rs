use conan2::{ConanInstall, ConanVerbosity};

fn main() {
    ConanInstall::new()
        .build("missing")
        .verbosity(ConanVerbosity::Error) // Silence Conan warnings
        .run()
        .parse()
        .emit();
}
