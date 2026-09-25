import QtQuick
import org.blossomos.sangria
import org.kde.coreaddons as Core
import org.kde.kirigamiaddons.formcard as FormCard

FormCard.AboutPage {
    id: page

    aboutData: ({
        displayName: Core.AboutData.displayName,
        productName: "sangria",
        componentName: Core.AboutData.componentName,
        shortDescription: Core.AboutData.shortDescription,
        version: Core.AboutData.version,
        otherText: qsTr("Runs Windows programs on Linux."),
        copyrightStatement: "© 2026 Blossom Labs",
        homepage: "https://blossom.computer",
        bugAddress: "https://dev.blossomos.org/blossom/sangria/-/boards",
        desktopFileName: "org.blossomos.sangria",
        programLogo: "qrc:/qt/qml/org/blossomos/sangria/org.blossomos.sangria.svg",
        authors: [],
        credits: [],
        translators: [],
        licenses: [{
            name: "GNU Affero General Public License v3.0 or later",
            text: LicenseText.text
        }]
    })

    donateUrl: "https://blossom.computer/donate"
    getInvolvedUrl: "https://dev.blossomos.org/blossom/sangria"
    showLibraries: true
}
