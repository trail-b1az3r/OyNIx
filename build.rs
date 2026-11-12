use cxx_qt_build::{CxxQtBuilder, QResource, QResources, QmlFile, QmlModule};
use qt_build_utils::QResourceFile;
use std::path::{Path, PathBuf};

fn collect(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            collect(&p, ext, out);
        } else if p.extension().map(|e| e == ext).unwrap_or(false) {
            out.push(p);
        }
    }
}

fn main() {
    let mut qml = Vec::new();
    collect(Path::new("qml"), "qml", &mut qml);
    let qml_files: Vec<QmlFile> = qml
        .iter()
        .map(|p| {
            let singleton = std::fs::read_to_string(p).map(|s| s.contains("pragma Singleton")).unwrap_or(false);
            QmlFile::from(p).singleton(singleton)
        })
        .collect();

    // QML files live in qml/, which Qt imports implicitly as a directory.
    // Give that directory its own qmldir so the Theme singleton resolves
    // as a singleton there too.
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let mut qmldir = String::new();
    for p in &qml {
        let name = p.file_stem().unwrap().to_string_lossy();
        let file = p.file_name().unwrap().to_string_lossy();
        let singleton = std::fs::read_to_string(p).map(|s| s.contains("pragma Singleton")).unwrap_or(false);
        qmldir.push_str(&format!("{}{name} 1.0 {file}\n", if singleton { "singleton " } else { "" }));
    }
    let qmldir_path = out.join("qml-dir-qmldir");
    std::fs::write(&qmldir_path, qmldir).unwrap();

    let mut resources: Vec<PathBuf> = Vec::new();
    collect(Path::new("qml/icons"), "svg", &mut resources);
    resources.push("assets/icon.png".into());
    resources.push("assets/icon.svg".into());
    resources.push("assets/logo.svg".into());

    println!("cargo::rerun-if-changed=qml");
    println!("cargo::rerun-if-changed=cpp");

    CxxQtBuilder::new_qml_module(QmlModule::new("OyNIx").qml_files(qml_files))
        .qt_module("Gui")
        .qt_module("Quick")
        .qt_module("WebEngineCore")
        .qt_module("WebEngineQuick")
        .files(["src/bridge/core.rs", "src/bridge/native.rs"])
        .cpp_file("cpp/native.cpp")
        .qrc_resources(resources.iter().map(|p| p.to_string_lossy().into_owned()).collect::<Vec<_>>())
        .qrc_resources(QResources::new().resource(
            QResource::new().prefix("/qt/qml/OyNIx").file(QResourceFile::new(&qmldir_path).alias("qml/qmldir")),
        ))
        .build();
}
