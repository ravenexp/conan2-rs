from conan import ConanFile


class TestSystemLibsConan(ConanFile):
    """
    Defines a Conan package for the cpp_info.system_libs attribute tests.
    """

    name = "system_libs_test"
    version = "0.1.0"

    package_type = "static-library"
    build_policy = "missing"

    settings = "os", "compiler", "build_type", "arch"

    def package_info(self):
        self.cpp_info.system_libs = ["libyaml.so", "liby.a"]
