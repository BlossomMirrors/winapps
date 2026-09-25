import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as Controls
import org.blossomos.sangria
import org.kde.kirigami as Kirigami

Kirigami.ApplicationWindow {
    id: root

    title: qsTr("Sangria")

    minimumWidth: Kirigami.Units.gridUnit * 30
    minimumHeight: Kirigami.Units.gridUnit * 24
    width: Kirigami.Units.gridUnit * 44
    height: Kirigami.Units.gridUnit * 32

    Component { id: aboutPage; AboutPage { } }
    Component { id: debugPage; DebugPage { } }
    Component { id: settingsPage; SettingsPage { } }

    Component { id: importWindowComponent; ImportWindow { } }

    Controls.Popup {
        id: updateScreen

        parent: Controls.Overlay.overlay
        anchors.centerIn: parent
        modal: true
        closePolicy: Controls.Popup.NoAutoClose
        visible: Library.preparing
        padding: Kirigami.Units.largeSpacing * 2

        ColumnLayout {
            spacing: Kirigami.Units.largeSpacing

            Controls.ProgressBar {
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: Kirigami.Units.gridUnit * 22
                indeterminate: Library.progress < 0
                from: 0
                to: 1
                value: Library.progress
            }

            Kirigami.Heading {
                Layout.alignment: Qt.AlignHCenter
                level: 3
                text: qsTr("Updating the compatibility layer")
            }

            Controls.Label {
                Layout.alignment: Qt.AlignHCenter
                Layout.maximumWidth: Kirigami.Units.gridUnit * 26
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
                text: Library.status.length > 0
                    ? Library.status
                    : qsTr("Checking for a newer build…")
            }

            Controls.Label {
                Layout.alignment: Qt.AlignHCenter
                Layout.maximumWidth: Kirigami.Units.gridUnit * 26
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
                color: Kirigami.Theme.disabledTextColor
                font: Kirigami.Theme.smallFont
                text: qsTr("The app starts as soon as this is done.")
            }
        }
    }

    pageStack.initialPage: LibraryPage {
        onImportRequested: importWindowComponent.createObject(root).open()
    }

    globalDrawer: Kirigami.GlobalDrawer {
        isMenu: true
        actions: [
            Kirigami.Action {
                icon.name: "settings-configure"
                text: qsTr("Settings")
                onTriggered: pageStack.pushDialogLayer(settingsPage, {}, {
                    width: Kirigami.Units.gridUnit * 34,
                    height: Kirigami.Units.gridUnit * 24,
                    title: qsTr("Settings")
                })
            },
            Kirigami.Action {
                icon.name: "tools-report-bug-symbolic"
                text: qsTr("Debugging")
                onTriggered: pageStack.pushDialogLayer(debugPage, {}, {
                    width: Kirigami.Units.gridUnit * 36,
                    height: Kirigami.Units.gridUnit * 38,
                    title: qsTr("Debugging")
                })
            },
            Kirigami.Action {
                icon.name: "help-about-symbolic"
                text: qsTr("About Sangria")
                onTriggered: pageStack.pushDialogLayer(aboutPage, {
                    width: root.width
                }, {
                    width: Kirigami.Units.gridUnit * 33,
                    height: Kirigami.Units.gridUnit * 35,
                    title: qsTr("About Sangria")
                })
            }
        ]
    }
}
