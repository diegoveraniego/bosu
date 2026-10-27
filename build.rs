fn main() {
    for path in [
        "data/resources.gresource.xml",
        "data/icons/hicolor/scalable/apps/org.diego.Bosu.svg",
        "data/icons/hicolor/symbolic/apps/org.diego.Bosu-symbolic.svg",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }
    let target = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo OUT_DIR"))
        .join("bosu.gresource");
    let status = std::process::Command::new("glib-compile-resources")
        .arg("data/resources.gresource.xml")
        .arg("--sourcedir=data")
        .arg("--target")
        .arg(target)
        .status()
        .expect("Install glib-compile-resources (GLib development tools)");
    assert!(status.success(), "Could not compile Bosu resources");
}
