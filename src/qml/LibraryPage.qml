import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as Controls
import QtQuick.Dialogs
import org.blossomos.winapps
import org.kde.kirigami as Kirigami
import org.kde.kirigamiaddons.formcard as FormCard

Kirigami.ScrollablePage {
    id: page

    title: qsTr("Library")

    readonly property var entries: Library.entriesJson.length > 0
        ? JSON.parse(Library.entriesJson)
        : []

    signal importRequested()

    actions: [
        Kirigami.Action {
            icon.name: "list-add-symbolic"
            text: qsTr("Import")
            enabled: !Library.busy
            onTriggered: importDialog.open()
        }
    ]

    header: Controls.ToolBar {
        visible: Library.busy
        contentItem: RowLayout {
            spacing: Kirigami.Units.largeSpacing

            Controls.BusyIndicator {
                running: Library.busy
                implicitWidth: Kirigami.Units.iconSizes.smallMedium
                implicitHeight: Kirigami.Units.iconSizes.smallMedium
            }

            Controls.Label {
                Layout.fillWidth: true
                elide: Text.ElideRight
                text: Library.status.length > 0 ? Library.status : qsTr("Working…")
            }

            Controls.Button {
                text: qsTr("Cancel")
                icon.name: "dialog-cancel"
                onClicked: Library.cancel()
            }
        }
    }

    FileDialog {
        id: importDialog
        title: qsTr("Pick a Windows program or installer")
        nameFilters: [
            qsTr("Windows programs and installers (*.exe *.msi)"),
            qsTr("Programs (*.exe)"),
            qsTr("Installer packages (*.msi)"),
            qsTr("All files (*)")
        ]
        onAccepted: {
            Library.prepare(selectedFile.toString())
            if (Library.stagedJson !== "null") {
                page.importRequested()
            }
        }
    }

    Kirigami.PlaceholderMessage {
        anchors.centerIn: parent
        width: parent.width - Kirigami.Units.gridUnit * 4
        visible: page.entries.length === 0
        icon.name: "application-x-ms-dos-executable"
        text: qsTr("No apps yet")
        explanation: qsTr("Import a Windows program to run it, or an installer to set it up first.")

        helpfulAction: Kirigami.Action {
            icon.name: "list-add-symbolic"
            text: qsTr("Import")
            onTriggered: importDialog.open()
        }
    }

    ColumnLayout {
        width: parent.width
        spacing: 0
        visible: page.entries.length > 0

        Repeater {
            model: page.entries

            FormCard.FormCard {
                required property var modelData

                Layout.topMargin: Kirigami.Units.largeSpacing

                FormCard.AbstractFormDelegate {
                    background: null
                    contentItem: RowLayout {
                        spacing: Kirigami.Units.largeSpacing

                        Kirigami.Icon {
                            source: modelData.icon.length > 0
                                ? "file://" + modelData.icon
                                : "application-x-ms-dos-executable"
                            Layout.preferredWidth: Kirigami.Units.iconSizes.large
                            Layout.preferredHeight: Kirigami.Units.iconSizes.large
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 0

                            Controls.Label {
                                Layout.fillWidth: true
                                text: modelData.name
                                elide: Text.ElideRight
                            }

                            Controls.Label {
                                Layout.fillWidth: true
                                elide: Text.ElideMiddle
                                color: Kirigami.Theme.disabledTextColor
                                font: Kirigami.Theme.smallFont
                                text: modelData.exe.length > 0
                                    ? modelData.exe
                                    : qsTr("No executable picked yet")
                            }
                        }

                        Controls.Button {
                            text: qsTr("Play")
                            icon.name: "media-playback-start-symbolic"
                            enabled: !Library.busy && modelData.exe.length > 0
                            onClicked: Library.launch(modelData.id)
                        }

                        Controls.ToolButton {
                            icon.name: "overflow-menu"
                            onClicked: appMenu.popup()

                            Controls.Menu {
                                id: appMenu

                                Controls.MenuItem {
                                    text: qsTr("Pick executable…")
                                    onTriggered: {
                                        exePicker.appId = modelData.id
                                        exePicker.options = JSON.parse(Library.candidates(modelData.id))
                                        exePicker.open()
                                    }
                                }
                                Controls.MenuItem {
                                    text: qsTr("Run installer again")
                                    enabled: !Library.busy && modelData.kind === "installer"
                                    onTriggered: Library.runInstaller(modelData.id)
                                }
                                Controls.MenuItem {
                                    text: modelData.shortcut
                                        ? qsTr("Remove desktop entry")
                                        : qsTr("Create desktop entry")
                                    onTriggered: Library.setShortcut(modelData.id, !modelData.shortcut)
                                }
                                Controls.MenuSeparator { }
                                Controls.MenuItem {
                                    text: qsTr("Run winetricks")
                                    enabled: !Library.busy
                                    onTriggered: Library.winetricks(modelData.id)
                                }
                                Controls.MenuItem {
                                    text: qsTr("Open prefix")
                                    onTriggered: Library.openPrefix(modelData.id)
                                }
                                Controls.MenuSeparator { }
                                Controls.MenuItem {
                                    text: qsTr("Uninstall…")
                                    icon.name: "edit-delete-symbolic"
                                    onTriggered: {
                                        uninstallPrompt.appId = modelData.id
                                        uninstallPrompt.appName = modelData.name
                                        uninstallPrompt.appPrefix = modelData.prefix
                                        uninstallPrompt.open()
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Kirigami.PromptDialog {
        id: uninstallPrompt

        property string appId: ""
        property string appName: ""
        property string appPrefix: ""

        title: qsTr("Uninstall %1?").arg(appName)
        subtitle: qsTr("This deletes the program, its Windows prefix and its desktop entry. Saved games and settings inside the prefix go with it.\n\n%1").arg(appPrefix)
        standardButtons: Kirigami.Dialog.Cancel

        footerLeadingComponent: Controls.Button {
            text: qsTr("Uninstall")
            icon.name: "edit-delete-symbolic"
            icon.color: Kirigami.Theme.negativeTextColor
            palette.buttonText: Kirigami.Theme.negativeTextColor
            onClicked: {
                Library.uninstall(uninstallPrompt.appId)
                uninstallPrompt.close()
            }
        }
    }

    Kirigami.Dialog {
        id: exePicker

        property string appId: ""
        property var options: []

        title: qsTr("Pick the executable to launch")
        standardButtons: Kirigami.Dialog.Cancel
        preferredWidth: Kirigami.Units.gridUnit * 30

        ColumnLayout {
            spacing: 0

            Controls.Label {
                Layout.fillWidth: true
                Layout.margins: Kirigami.Units.largeSpacing
                visible: exePicker.options.length === 0
                wrapMode: Text.WordWrap
                text: qsTr("Nothing found in the prefix. Run the installer first.")
            }

            Repeater {
                model: exePicker.options

                Controls.ItemDelegate {
                    required property string modelData

                    Layout.fillWidth: true
                    text: modelData
                    onClicked: {
                        Library.setExe(exePicker.appId, modelData)
                        exePicker.close()
                    }
                }
            }
        }
    }
}
