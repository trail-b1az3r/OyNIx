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

/// Minimum Qt: 6.10 (Qt WebEngine extension manager).
const MIN_QT: (u32, u32) = (6, 10);

/// Pick the Qt installation: $QMAKE, else a Qt fetched into ./.qt by
/// scripts/fetch-qt.sh (newest version wins), else the system qmake6.
fn select_qmake() -> PathBuf {
    if let Ok(q) = std::env::var("QMAKE") {
        return PathBuf::from(q);
    }
    let mut local: Vec<PathBuf> = std::fs::read_dir(".qt")
        .map(|rd| rd.flatten().map(|e| e.path().join("gcc_64/bin/qmake")).filter(|p| p.exists()).collect())
        .unwrap_or_default();
    local.sort();
    if let Some(q) = local.pop() {
        return q;
    }
    PathBuf::from("qmake6")
}

fn qmake_query(qmake: &Path, key: &str) -> String {
    std::process::Command::new(qmake)
        .args(["-query", key])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn main() {
    println!("cargo::rerun-if-env-changed=QMAKE");
    println!("cargo::rerun-if-env-changed=OYNIX_VERSION");
    println!("cargo::rerun-if-changed=.qt");
    let qmake = select_qmake();
    let qmake = std::fs::canonicalize(&qmake).unwrap_or(qmake);
    let version = qmake_query(&qmake, "QT_VERSION");
    let mut parts = version.split('.').map(|p| p.parse::<u32>().unwrap_or(0));
    let (maj, min) = (parts.next().unwrap_or(0), parts.next().unwrap_or(0));
    if (maj, min) < MIN_QT {
        panic!(
            "OyNIx needs Qt >= {}.{} (found '{}' via {}).\n\
             Run ./scripts/fetch-qt.sh to download Qt 6.11 into ./.qt, or set QMAKE=/path/to/qmake.",
            MIN_QT.0, MIN_QT.1, version, qmake.display()
        );
    }
    // cxx-qt-build reads QMAKE from the environment of this process.
    std::env::set_var("QMAKE", &qmake);
    // Run against the same Qt we linked, wherever it lives.
    // OYNIX_RPATH overrides the runtime library path (packaging uses
    // `$ORIGIN/qt/lib` for a relocatable bundle).
    println!("cargo::rerun-if-env-changed=OYNIX_RPATH");
    let libs = qmake_query(&qmake, "QT_INSTALL_LIBS");
    if let Ok(rp) = std::env::var("OYNIX_RPATH") {
        println!("cargo::rustc-link-arg=-Wl,-rpath,{rp}");
    } else if !libs.is_empty() && !libs.starts_with("/usr/lib") {
        println!("cargo::rustc-link-arg=-Wl,-rpath,{libs}");
    }
    println!("cargo::rustc-env=OYNIX_QT_BUILD_VERSION={version}");

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
        .qt_module("Qml")
        .qt_module("WebEngineCore")
        .qt_module("WebEngineQuick")
        .qt_module("WebChannel")
        .qt_module("Network")
        .files(["src/bridge/agentbridge.rs", "src/bridge/core.rs", "src/bridge/extbridge.rs", "src/bridge/native.rs", "src/bridge/usbbridge.rs", "src/bridge/panel.rs"])
        .cpp_file("cpp/native.cpp")
        .qrc_resources(resources.iter().map(|p| p.to_string_lossy().into_owned()).collect::<Vec<_>>())
        .qrc_resources(QResources::new().resource(
            QResource::new().prefix("/qt/qml/OyNIx").file(QResourceFile::new(&qmldir_path).alias("qml/qmldir")),
        ))
        .build();
}
