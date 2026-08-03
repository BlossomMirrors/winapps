use cxx_qt_lib::{QByteArray, QGuiApplication, QQmlApplicationEngine, QQuickStyle, QString, QUrl};

use cxx_qt_lib_extras::QApplication;

use cxx_qt::casting::Upcast;

use cxx_kde_frameworks::kcrash::KCrash;

use cxx_kde_frameworks::kcoreaddons::{KAboutData, License};

use cxx_kde_frameworks::ki18n::{self, KLocalizedString, i18nc};

use std::env;

fn main() {
    let mut app = QApplication::new();

    KCrash::initialize();

    KLocalizedString::set_application_domain(&QByteArray::from("winapps"));

    // To associate the executable to the installed desktop file
    QGuiApplication::set_desktop_file_name(&QString::from("org.blossomos.winapps"));

    // To ensure the style is set correctly
    if env::var("QT_QUICK_CONTROLS_STYLE").is_err() {
        QQuickStyle::set_style(&QString::from("org.kde.desktop"));
    }

    let about_data = KAboutData::from(
        // componentName
        QString::from("winapps"),
        // displayName
        i18nc("@title", "Windows App Support"),
        // version
        QString::from("1.0"),
        // shortDescription
        QString::from("Windows App Support for BlossomOS"),
        // license
        License::MIT,
    );

    KAboutData::set_application_data(&about_data);

    let mut engine = QQmlApplicationEngine::new();

    if let Some(mut engine) = engine.as_mut() {
        ki18n::setup_localized_context(engine.as_mut().upcast_pin());

        engine.load(&QUrl::from(
            "qrc:/qt/qml/org/blossomos/winapps/src/qml/Main.qml",
        ));
    }

    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
