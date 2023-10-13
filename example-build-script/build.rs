use conan2::ConanInstall;

fn main() {
    ConanInstall::new().build("missing").run().parse().emit();
}
