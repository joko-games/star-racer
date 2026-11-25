//! Gives the Windows program its icon: the logo (`../logo/star-racer.ico`, rendered from
//! `star-racer.svg` by tools/icon) compiled in as a resource, so the executable, its shortcut and the
//! installer show it (docs/logo.md). Nothing to do for any other system, nor for the web.

fn main() {
    println!("cargo::rerun-if-changed=../logo/star-racer.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon("../logo/star-racer.ico")
        .set("ProductName", "Star Racer")
        .set("FileDescription", "Star Racer")
        .set("CompanyName", "Joko Games");
    resource.compile().expect("compiles the program's icon (needs the Windows SDK's rc.exe)");
}
