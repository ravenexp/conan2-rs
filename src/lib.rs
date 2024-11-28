//! # conan2-rs
//!
//! ## Introduction
//!
//! `conan2-rs` is a Cargo build script wrapper of the Conan C/C++ package manager
//! (version 2.0 only).
//!
//! It automatically pulls the C/C++ library linking flags from Conan dependencies
//! and passes them to `rustc`.
//!
//! ## Adding C/C++ dependencies using Conan
//!
//! The simplest way to add C/C++ dependencies to a Rust project using Conan
//! is to add a plain `conanfile.txt` file as follows:
//!
//! ```text
//! [requires]
//! libxml2/2.11.4
//! openssl/3.1.3
//! zlib/1.3
//! ```
//!
//! ## Example usage
//!
//! Add `conan2` to the `Cargo.toml` build dependencies section:
//!
//! ```toml
//! [build-dependencies]
//! conan2 = "0.1"
//! ```
//!
//! Add the following lines to the project `build.rs` script to invoke `conan install`
//! and pass the Conan dependency information to Cargo automatically:
//!
//! ```no_run
//! use conan2::ConanInstall;
//!
//! ConanInstall::new().run().parse().emit();
//! ```
//!
//! The most commonly used `build_type` Conan setting will be defined automatically
//! depending on the current Cargo build profile: Debug or Release.
//!
//! The Conan executable is assumed to be named `conan` unless
//! the `CONAN` environment variable is set to override.
//!
//! ## Advanced usage
//!
//! ### Automatic Conan profile inference from Cargo target
//!
//! Using custom Conan profiles with names derived from the Cargo target information
//! and a reduced output verbosity level:
//!
//! ```no_run
//! use conan2::{ConanInstall, ConanVerbosity};
//!
//! let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
//! let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
//! let conan_profile = format!("{}-{}", target_os, target_arch);
//!
//! ConanInstall::new()
//!     .profile(&conan_profile)
//!     .build("missing")
//!     .verbosity(ConanVerbosity::Error) // Silence Conan warnings
//!     .run()
//!     .parse()
//!     .emit();
//! ```
//!
//! ### Automatic Conan profile creation
//!
//! Creating a custom default Conan profile on the fly with zero configuration:
//!
//! ```no_run
//! use conan2::{ConanInstall, ConanVerbosity};
//!
//! ConanInstall::new()
//!     .profile("cargo")
//!     .detect_profile() // Run `conan profile detect --exist-ok` for the above
//!     .run()
//!     .parse()
//!     .emit();
//! ```
//!
//! ### Using separate host and build profiles
//!
//! Using different values for `--profile:host` and `--profile:build`
//! arguments of `conan install` command:
//!
//! ```no_run
//! use conan2::{ConanInstall, ConanVerbosity};
//!
//! ConanInstall::new()
//!     .host_profile("cargo-host")
//!     .build_profile("cargo-build")
//!     .run()
//!     .parse()
//!     .emit();
//! ```
//!
//! ### Getting C/C++ include paths from Conan dependencies
//!
//! To use the list of include paths, do the following after
//! parsing the `conan install` output:
//!
//! ```no_run
//! use conan2::ConanInstall;
//!
//! let metadata = ConanInstall::new().run().parse();
//!
//! for path in metadata.include_paths() {
//!     // Add "-I{path}" to CXXFLAGS or something.
//! }
//!
//! metadata.emit();
//! ```

#![deny(missing_docs)]

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::io::{BufRead, Cursor, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Map, Value};

/// Conan binary override environment variable
const CONAN_ENV: &str = "CONAN";

/// Default Conan binary name
const DEFAULT_CONAN: &str = "conan";

/// `conan` command verbosity level
///
/// Defines the level of detail of the Conan command output.
///
/// Enum variants correspond to the following options:
/// `-vquiet`, `-verror`, `-vwarning`, `-vnotice`, `-vstatus`,
/// `-v` or `-vverbose`, `-vv` or `-vdebug`, `-vvv` or `-vtrace`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConanVerbosity {
    /// `-vquiet`
    Quiet,
    /// `-verror`
    Error,
    /// `-vwarning`
    #[default]
    Warning,
    /// `-vnotice`
    Notice,
    /// `-vstatus`
    Status,
    /// `-vverbose`
    Verbose,
    /// `-vdebug`
    Debug,
    /// `-vtrace`
    Trace,
}

/// `conan install` command builder
///
/// This opaque type implements a command line builder for
/// the `conan install` command invocation.
#[derive(Default)]
pub struct ConanInstall {
    /// Conan generators output directory
    output_folder: Option<PathBuf>,
    /// Conan recipe file path
    recipe_path: Option<PathBuf>,
    /// Conan host profile name
    profile: Option<String>,
    /// Conan build profile name
    build_profile: Option<String>,
    /// Conan profile auto-detection flag
    new_profile: bool,
    /// Conan build policy
    build: Option<String>,
    /// Conan output verbosity level
    verbosity: ConanVerbosity,
}

/// `conan install` command output data
pub struct ConanOutput(Output);

/// Build script instructions for Cargo
pub struct CargoInstructions {
    /// Raw build script output
    out: Vec<u8>,
    /// C include paths collected from the packages
    includes: BTreeSet<PathBuf>,
}

/// Conan dependency graph as a JSON-based tree structure
struct ConanDependencyGraph(Value);

impl std::fmt::Display for ConanVerbosity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConanVerbosity::Quiet => f.write_str("quiet"),
            ConanVerbosity::Error => f.write_str("error"),
            ConanVerbosity::Warning => f.write_str("warning"),
            ConanVerbosity::Notice => f.write_str("notice"),
            ConanVerbosity::Status => f.write_str("status"),
            ConanVerbosity::Verbose => f.write_str("verbose"),
            ConanVerbosity::Debug => f.write_str("debug"),
            ConanVerbosity::Trace => f.write_str("trace"),
        }
    }
}

impl ConanInstall {
    /// Creates a new `conan install` command with the default recipe path (`.`).
    #[must_use]
    pub fn new() -> ConanInstall {
        ConanInstall::default()
    }

    /// Creates a new `conan install` command with the user-provided recipe path.
    #[must_use]
    pub fn with_recipe(recipe_path: &Path) -> ConanInstall {
        ConanInstall {
            recipe_path: Some(recipe_path.to_owned()),
            ..Default::default()
        }
    }

    /// Sets a custom Conan generator output folder path.
    ///
    /// Matches `--output-folder` Conan executable option.
    ///
    /// This method should not be used in most cases:
    ///
    /// The Cargo-provided `OUT_DIR` environment variable value is used
    /// as the default when the command is invoked from a build script.
    pub fn output_folder(&mut self, output_folder: &Path) -> &mut ConanInstall {
        self.output_folder = Some(output_folder.to_owned());
        self
    }

    /// Sets the Conan profile name to use for installing dependencies.
    ///
    /// Matches `--profile` Conan executable option.
    pub fn profile(&mut self, profile: &str) -> &mut ConanInstall {
        self.profile = Some(profile.to_owned());
        self
    }

    /// Sets the Conan host profile name to use for installing dependencies.
    ///
    /// Matches `--profile:host` Conan executable option.
    pub fn host_profile(&mut self, profile: &str) -> &mut ConanInstall {
        self.profile(profile)
    }

    /// Sets the Conan build profile name to use for installing dependencies.
    ///
    /// Matches `--profile:build` Conan executable option.
    pub fn build_profile(&mut self, profile: &str) -> &mut ConanInstall {
        self.build_profile = Some(profile.to_owned());
        self
    }

    /// Auto-detects and creates the Conan profile to use for installing dependencies.
    ///
    /// Schedules `conan profile detect --exist-ok` to run before running `conan install`.
    pub fn detect_profile(&mut self) -> &mut ConanInstall {
        self.new_profile = true;
        self
    }

    /// Sets the Conan dependency build policy for `conan install`.
    ///
    /// Matches `--build` Conan executable option.
    pub fn build(&mut self, build: &str) -> &mut ConanInstall {
        self.build = Some(build.to_owned());
        self
    }

    /// Sets the Conan command verbosity level.
    ///
    /// Matches `-v` Conan executable option.
    pub fn verbosity(&mut self, verbosity: ConanVerbosity) -> &mut ConanInstall {
        self.verbosity = verbosity;
        self
    }

    /// Runs the `conan install` command and captures its JSON-formatted output.
    ///
    /// # Panics
    ///
    /// Panics if the Conan executable cannot be found.
    #[must_use]
    pub fn run(&self) -> ConanOutput {
        let conan = std::env::var_os(CONAN_ENV).unwrap_or_else(|| DEFAULT_CONAN.into());
        let recipe = self.recipe_path.as_deref().unwrap_or(Path::new("."));

        let output_folder = match &self.output_folder {
            Some(s) => s.clone(),
            None => std::env::var_os("OUT_DIR")
                .expect("OUT_DIR environment variable must be set")
                .into(),
        };

        if self.new_profile {
            Self::run_profile_detect(&conan, self.profile.as_deref());

            if self.build_profile != self.profile {
                Self::run_profile_detect(&conan, self.build_profile.as_deref());
            };
        }

        let mut command = Command::new(conan);
        command
            .arg("install")
            .arg(recipe)
            .arg(format!("-v{}", self.verbosity))
            .arg("--format")
            .arg("json")
            .arg("--output-folder")
            .arg(output_folder);

        if let Some(profile) = self.profile.as_deref() {
            command.arg("--profile:host").arg(profile);
        }

        if let Some(build_profile) = self.build_profile.as_deref() {
            command.arg("--profile:build").arg(build_profile);
        }

        if let Some(build) = self.build.as_deref() {
            command.arg("--build");
            command.arg(build);
        }

        // Use additional environment variables set by Cargo.
        Self::add_settings_from_env(&mut command);

        let output = command
            .output()
            .expect("failed to run the Conan executable");

        ConanOutput(output)
    }

    /// Creates a new profile with `conan profile detect` if required.
    fn run_profile_detect(conan: &OsStr, profile: Option<&str>) {
        let mut command = Command::new(conan);
        command.arg("profile").arg("detect").arg("--exist-ok");

        if let Some(profile) = profile {
            println!("running 'conan profile detect' for profile '{profile}'");

            command.arg("--name").arg(profile);
        } else {
            println!("running 'conan profile detect' for the default profile");
        }

        let status = command
            .status()
            .expect("failed to run the Conan executable");

        #[allow(clippy::manual_assert)]
        if !status.success() {
            panic!("'conan profile detect' command failed: {status}");
        }
    }

    /// Adds automatic Conan settings arguments derived
    /// from the environment variables set by Cargo.
    ///
    /// The following Conan settings are auto-detected and set:
    ///
    /// - `build_type`
    fn add_settings_from_env(command: &mut Command) {
        match std::env::var("PROFILE").as_deref() {
            Ok("debug") => {
                command.arg("-s");
                command.arg("build_type=Debug");
            }
            Ok("release") => {
                command.arg("-s");
                command.arg("build_type=Release");
            }
            _ => (),
        }
    }
}

impl ConanOutput {
    /// Parses `conan install` command output and generates build script
    /// instructions for Cargo.
    ///
    /// # Panics
    ///
    /// Panics if the Conan command invocation failed or
    /// the JSON-formatted Conan output could not be parsed.
    #[must_use]
    pub fn parse(self) -> CargoInstructions {
        // Panic if the `conan install` command has failed.
        self.ensure_success();

        let mut cargo = CargoInstructions::new();

        // Re-run the build script if `CONAN` environment variable changes.
        cargo.rerun_if_env_changed(CONAN_ENV);

        // Pass Conan warnings through to Cargo using build script instructions.
        for line in Cursor::new(self.stderr()).lines() {
            if let Some(msg) = line.unwrap().strip_prefix("WARN: ") {
                cargo.warning(msg);
            }
        }

        // Parse the JSON-formatted `conan install` command output.
        let metadata: Value =
            serde_json::from_slice(self.stdout()).expect("failed to parse JSON output");

        // Walk the dependency graph and collect the C/C++ libraries.
        ConanDependencyGraph(metadata).traverse(&mut cargo);

        cargo
    }

    /// Ensures that the Conan command has been executed successfully.
    ///
    /// # Panics
    ///
    /// Panics with an error message if the Conan command invocation failed.
    pub fn ensure_success(&self) {
        if self.is_success() {
            return;
        }

        let code = self.status_code();
        let msg = String::from_utf8_lossy(self.stderr());

        panic!("Conan failed with status {code}: {msg}");
    }

    /// Checks the Conan install command execution status.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.0.status.success()
    }

    /// Gets the Conan install command execution status code.
    #[must_use]
    pub fn status_code(&self) -> i32 {
        self.0.status.code().unwrap_or_default()
    }

    /// Gets the Conan JSON-formatted output as bytes.
    #[must_use]
    pub fn stdout(&self) -> &[u8] {
        &self.0.stdout
    }

    /// Gets the Conan command error message as bytes.
    #[must_use]
    pub fn stderr(&self) -> &[u8] {
        &self.0.stderr
    }
}

impl CargoInstructions {
    /// Emits build script instructions for Cargo into `stdout`.
    ///
    /// # Panics
    ///
    /// Panics if the Cargo build instructions can not be written to `stdout`.
    pub fn emit(&self) {
        std::io::stdout().write_all(self.as_bytes()).unwrap();
    }

    /// Gets the Cargo instruction lines as bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.out
    }

    /// Gets the C/C++ include directory paths for all dependencies.
    #[must_use]
    pub fn include_paths(&self) -> Vec<PathBuf> {
        self.includes.iter().cloned().collect()
    }

    /// Creates a new empty Cargo instructions list.
    fn new() -> CargoInstructions {
        CargoInstructions {
            out: Vec::with_capacity(1024),
            includes: BTreeSet::new(),
        }
    }

    /// Adds `cargo:warning={message}` instruction.
    fn warning(&mut self, message: &str) {
        writeln!(self.out, "cargo:warning={message}").unwrap();
    }

    /// Adds `cargo:rerun-if-env-changed={val}` instruction.
    fn rerun_if_env_changed(&mut self, val: &str) {
        writeln!(self.out, "cargo:rerun-if-env-changed={val}").unwrap();
    }

    /// Adds `cargo:rustc-link-args-bins={val}` instruction.
    fn rustc_link_arg_bins(&mut self, val: &str) {
        writeln!(self.out, "cargo:rustc-link-arg-bins={val}").unwrap();
    }

    /// Adds `cargo:rustc-link-lib={lib}` instruction.
    fn rustc_link_lib(&mut self, lib: &str) {
        writeln!(self.out, "cargo:rustc-link-lib={lib}").unwrap();
    }

    /// Adds `cargo:rustc-link-search={path}` instruction.
    fn rustc_link_search(&mut self, path: &str) {
        writeln!(self.out, "cargo:rustc-link-search={path}").unwrap();
    }

    /// Adds `cargo:include={path}` instruction.
    fn include(&mut self, path: &str) {
        writeln!(self.out, "cargo:include={path}").unwrap();
        self.includes.insert(path.into());
    }
}

impl ConanDependencyGraph {
    /// Traverses the dependency graph and emits the `rustc` link instructions
    /// in the correct linking order.
    fn traverse(self, cargo: &mut CargoInstructions) {
        // Consumer package node id: the root of the graph
        let root_node_id = "0";

        self.visit_dependency(cargo, root_node_id);
    }

    /// Visits the dependencies recursively starting from node `node_id`
    /// and emits `rustc` link instructions.
    fn visit_dependency(&self, cargo: &mut CargoInstructions, node_id: &str) {
        let Some(node) = self.find_node(node_id) else {
            return;
        };

        if let Some(Value::Object(cpp_info)) = node.get("cpp_info") {
            for cpp_comp_name in cpp_info.keys() {
                Self::visit_cpp_component(cargo, cpp_info, cpp_comp_name);
            }
        };

        // Recursively visit transitive dependencies.
        if let Some(Value::Object(dependencies)) = node.get("dependencies") {
            for dependency_id in dependencies.keys() {
                self.visit_dependency(cargo, dependency_id);
            }
        };
    }

    /// Visits the dependency package components recursively starting from
    /// the component named `comp_name` and emits `rustc` link instructions.
    fn visit_cpp_component(
        cargo: &mut CargoInstructions,
        cpp_info: &Map<String, Value>,
        comp_name: &str,
    ) {
        let Some(component) = Self::find_cpp_component(cpp_info, comp_name) else {
            return;
        };

        // Skip dependency components which provide no C/C++ libraries.
        let Some(Value::Array(libs)) = component.get("libs") else {
            return;
        };
        if libs.is_empty() {
            return;
        }

        // Skip dependency components which provide no library paths.
        let Some(Value::Array(libdirs)) = component.get("libdirs") else {
            return;
        };

        // 1. Emit linker search directory instructions for `rustc`.
        for libdir in libdirs {
            if let Value::String(libdir) = libdir {
                cargo.rustc_link_search(libdir);
            }
        }

        // 2. Emit library link instructions for `rustc`.
        for lib in libs {
            if let Value::String(lib) = lib {
                cargo.rustc_link_lib(lib);
            }
        }

        // 3. Emit system library link instructions for `rustc`.
        if let Some(Value::Array(system_libs)) = component.get("system_libs") {
            for system_lib in system_libs {
                if let Value::String(system_lib) = system_lib {
                    cargo.rustc_link_lib(system_lib);
                }
            }
        };

        // 4. Emit "cargo:include=DIR" metadata for Rust dependencies.
        if let Some(Value::Array(includedirs)) = component.get("includedirs") {
            for include in includedirs {
                if let Value::String(include) = include {
                    cargo.include(include);
                }
            }
        };

        // 5. Emit "cargo:rustc-link-arg-bins=FLAGS" metadata for `rustc`.
        if let Some(Value::Array(flags)) = component.get("exelinkflags") {
            for flag in flags {
                if let Value::String(flag) = flag {
                    cargo.rustc_link_arg_bins(flag);
                }
            }
        }

        // 6. Recursively visit dependency component requirements.
        if let Some(Value::Array(requires)) = component.get("requires") {
            for requirement in requires {
                if let Value::String(req_comp_name) = requirement {
                    Self::visit_cpp_component(cargo, cpp_info, req_comp_name);
                }
            }
        };
    }

    /// Gets the dependency node field map by the node `id` key.
    fn find_node(&self, id: &str) -> Option<&Map<String, Value>> {
        let Value::Object(root) = &self.0 else {
            panic!("root JSON object expected");
        };

        let Some(Value::Object(graph)) = root.get("graph") else {
            panic!("root 'graph' object expected");
        };

        let Some(Value::Object(nodes)) = graph.get("nodes") else {
            panic!("root 'nodes' object expected");
        };

        if let Some(Value::Object(node)) = nodes.get(id) {
            Some(node)
        } else {
            None
        }
    }

    /// Gets the dependency component field map by its name.
    fn find_cpp_component<'a>(
        cpp_info: &'a Map<String, Value>,
        name: &str,
    ) -> Option<&'a Map<String, Value>> {
        if let Some(Value::Object(component)) = cpp_info.get(name) {
            Some(component)
        } else {
            None
        }
    }
}
