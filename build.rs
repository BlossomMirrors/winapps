use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(
        QmlModule::new("org.blossomos.winapps").qml_file("src/qml/Main.qml"),
    )
    .build();
}
