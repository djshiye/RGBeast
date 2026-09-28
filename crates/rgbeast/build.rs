use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    let data = PathBuf::from("../../data");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let ui_out = out.join("ui");
    fs::create_dir_all(&ui_out).unwrap();

    let mut blps = Vec::new();
    for entry in fs::read_dir(data.join("ui")).unwrap() {
        let p = entry.unwrap().path();
        if p.extension().is_some_and(|e| e == "blp") {
            println!("cargo:rerun-if-changed={}", p.display());
            blps.push(p);
        }
    }
    let status = Command::new("blueprint-compiler")
        .arg("batch-compile")
        .arg(&ui_out)
        .arg(data.join("ui"))
        .args(&blps)
        .status()
        .expect("blueprint-compiler not found (dnf install blueprint-compiler)");
    assert!(status.success(), "blueprint-compiler failed");

    // GSettings schema for uninstalled runs (debug builds point GSETTINGS_SCHEMA_DIR here).
    let schema_dir = out.join("schemas");
    fs::create_dir_all(&schema_dir).unwrap();
    let schema = "io.github.djshiye.RGBeast.gschema.xml";
    fs::copy(data.join(schema), schema_dir.join(schema)).unwrap();
    println!("cargo:rerun-if-changed={}", data.join(schema).display());
    let status = Command::new("glib-compile-schemas")
        .arg(&schema_dir)
        .status()
        .expect("glib-compile-schemas not found");
    assert!(status.success(), "glib-compile-schemas failed");

    println!(
        "cargo:rerun-if-changed={}",
        data.join("rgbeast.gresource.xml").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        data.join("style.css").display()
    );
    glib_build_tools::compile_resources(
        &[data.to_str().unwrap(), out.to_str().unwrap()],
        data.join("rgbeast.gresource.xml").to_str().unwrap(),
        "rgbeast.gresource",
    );
}
