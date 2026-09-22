mod library;
mod proton;
mod wine;

use cxx_qt_lib::{QByteArray, QGuiApplication, QQmlApplicationEngine, QQuickStyle, QString, QUrl};

use cxx_qt_lib_extras::QApplication;

use cxx_qt::casting::Upcast;

use cxx_kde_frameworks::kcrash::KCrash;

use cxx_kde_frameworks::kcoreaddons::{KAboutData, License};

use cxx_kde_frameworks::ki18n::{self, KLocalizedString, i18nc};

use std::env;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(QString, text)]
        type LicenseText = super::LicenseTextRust;
    }
}

pub struct LicenseTextRust {
    text: QString,
}

impl Default for LicenseTextRust {
    fn default() -> Self {
        Self {
            text: QString::from(include_str!("../LICENSE")),
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if let Some(index) = args.iter().position(|a| a == "--launch") {
        let Some(id) = args.get(index + 1) else {
            eprintln!("winapps: --launch needs an app id");
            std::process::exit(2);
        };
        std::process::exit(library::launch_headless(id));
    }

    // qt.svg is stupid
    let existing_rules = env::var("QT_LOGGING_RULES").unwrap_or_default();
    unsafe {
        env::set_var(
            "QT_LOGGING_RULES",
            format!("{existing_rules};qt.svg.warning=false;qt.qpa.services.warning=false")
                .trim_start_matches(';'),
        );
    }

    let mut app = QApplication::new();

    KCrash::initialize();

    KLocalizedString::set_application_domain(&QByteArray::from("winapps"));

    if env::var("QT_QUICK_CONTROLS_STYLE").is_err() {
        QQuickStyle::set_style(&QString::from("org.kde.desktop"));
    }

    let about_data = KAboutData::from(
        QString::from("winapps"),
        i18nc("@title", "WinApps"),
        QString::from("0.1.0"),
        QString::from("Install and run Windows programs on Linux"),
        License::Unknown,
    );

    KAboutData::set_application_data(&about_data);

    QGuiApplication::set_desktop_file_name(&QString::from("org.blossomos.winapps"));

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
