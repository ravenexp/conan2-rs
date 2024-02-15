# conan2-rs

## Introduction

`conan2-rs` is a Cargo build script wrapper of the Conan C/C++ package manager
(version 2.0 only).

It automatically pulls the C/C++ library linking flags from Conan dependencies
and passes them to `rustc`.

## Adding C/C++ dependencies using Conan

The simplest way to add C/C++ dependencies to a Rust project using Conan
is to add a plain `conanfile.txt` file as follows:

```text
[requires]
libxml2/2.11.4
openssl/3.1.3
zlib/1.3
```

## Example usage

Add `conan2` to the `Cargo.toml` build dependencies section:

```toml
[build-dependencies]
conan2 = "0.1"
```

Add the following lines to the project `build.rs` script to invoke `conan install`
and pass the Conan dependency information to Cargo automatically:

```rust
use conan2::ConanInstall;

fn main() {
    ConanInstall::new().run().parse().emit();
}
```

The most commonly used `build_type` Conan setting will be defined automatically
depending on the current Cargo build profile: Debug or Release.

The Conan executable is assumed to be named `conan` unless
the `CONAN` environment variable is set to override.

An example Rust crate using `conan2-rs` to link Conan dependencies
can also be found in the project repository.

## Advanced usage

Using custom Conan profiles with names derived from the Cargo target information:

```rust
use conan2::ConanInstall;

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let conan_profile = format!("{}-{}", target_os, target_arch);

    ConanInstall::new()
        .profile(&conan_profile)
        .build("missing")
        .run()
        .parse()
        .emit();
}
```

## Getting C/C++ include paths from Conan dependencies

To use the list of include paths, do the following after
parsing the `conan install` output:

```rust
use conan2::ConanInstall;

let metadata = ConanInstall::new().run().parse();

for path in metadata.include_paths() {
    // Add "-I{path}" to CXXFLAGS or something.
}

metadata.emit();
```
