use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    // GCC 16 added -Wsfinae-incomplete, which fires on Qt6's own qchar.h every
    // time it is forward-declared before being fully defined, a pattern Qt's
    // headers use throughout QtCore. It is a diagnostic about Qt's headers, not
    // about this crate, so it is disabled for the C++ this build compiles. The
    // `cc` crate reads CXXFLAGS from the environment, so it is read back here
    // first to avoid clobbering anything the caller already set.
    let cxxflags = std::env::var("CXXFLAGS").unwrap_or_default();
    unsafe {
        std::env::set_var(
            "CXXFLAGS",
            format!("{cxxflags} -Wno-sfinae-incomplete").trim(),
        );
    }

    CxxQtBuilder::new_qml_module(QmlModule::new("org.blossomos.sangria").qml_files([
        "src/qml/Main.qml",
        "src/qml/LibraryPage.qml",
        "src/qml/DebugPage.qml",
        "src/qml/ImportWindow.qml",
        "src/qml/SettingsPage.qml",
        "src/qml/AboutPage.qml",
    ]))
    .files(["src/main.rs", "src/library.rs"])
    .qrc_resources(["org.blossomos.sangria.svg"])
    .build();
}
