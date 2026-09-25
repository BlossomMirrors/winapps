import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as Controls
import QtQuick.Dialogs
import org.blossomos.sangria
import org.kde.kirigami as Kirigami
import org.kde.kirigamiaddons.formcard as FormCard

Kirigami.ApplicationWindow {
    id: window

    readonly property var staged: Library.stagedJson !== "null"
        ? JSON.parse(Library.stagedJson)
        : null
    property bool installing: false
    property bool asInstaller: true

    title: installing ? qsTr("Installing") : qsTr("Import")

    width: Kirigami.Units.gridUnit * 32
    height: Kirigami.Units.gridUnit * 30
    minimumWidth: Kirigami.Units.gridUnit * 26
    minimumHeight: Kirigami.Units.gridUnit * 22

    flags: Qt.Dialog

    function commit() {
        if (asInstaller) {
            window.installing = true
            Library.commitInstaller(nameField.text, rootButton.path,
                descriptionField.text, categoryBox.currentValue,
                gameIdField.store, shortcutSwitch.checked)
        } else {
            Library.commitPortable(nameField.text, descriptionField.text,
                categoryBox.currentValue, gameIdField.store, shortcutSwitch.checked)
            window.close()
        }
    }

    function open() {
        installing = false
        nameField.text = staged ? staged.name : ""
        descriptionField.text = staged ? staged.description : ""
        categoryBox.currentIndex = 0
        gameIdField.text = staged ? staged.gameid : ""
        gameIdField.store = staged ? staged.store : "none"
        asInstaller = !staged || staged.kind === "installer"
        kindBox.currentIndex = asInstaller ? 0 : 1
        rootButton.path = Library.prefixRoot
        Library.clearLog()
        show()
        raise()
        requestActivate()
    }

    Connections {
        target: Library
        function onStagedJsonChanged() {
            if (window.installing && Library.stagedJson === "null") {
                window.installing = false
                window.close()
            }
        }
    }

    FolderDialog {
        id: rootDialog
        title: qsTr("Where should the Windows prefix live?")
        currentFolder: "file://" + rootButton.path
        onAccepted: rootButton.path = selectedFolder.toString().replace("file://", "")
    }

    onClosing: close => {
        if (installing) {
            Library.cancel()
        } else if (staged !== null) {
            Library.discardStaged()
        }
        window.destroy()
    }

    pageStack.initialPage: FormCard.FormCardPage {
        id: importPage

        title: window.installing ? qsTr("Installing") : qsTr("Import")

        // Kirigami.Action has no "primary" button
        footer: Controls.ToolBar {
            visible: !window.installing
            contentItem: RowLayout {
                Item { Layout.fillWidth: true }

                Controls.Button {
                    icon.name: window.asInstaller ? "run-install-symbolic" : "list-add-symbolic"
                    text: window.asInstaller ? qsTr("Install") : qsTr("Add to library")
                    enabled: !Library.busy && nameField.text.length > 0
                    highlighted: true
                    onClicked: window.commit()
                }
            }
        }

        FormCard.FormCard {
            Layout.topMargin: Kirigami.Units.largeSpacing

            FormCard.AbstractFormDelegate {
                background: null
                contentItem: RowLayout {
                    spacing: Kirigami.Units.largeSpacing

                    Kirigami.Icon {
                        source: window.staged && window.staged.icon.length > 0
                            ? "file://" + window.staged.icon
                            : "application-x-ms-dos-executable"
                        Layout.preferredWidth: Kirigami.Units.iconSizes.huge
                        Layout.preferredHeight: Kirigami.Units.iconSizes.huge
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 0

                        Kirigami.Heading {
                            Layout.fillWidth: true
                            level: 2
                            elide: Text.ElideRight
                            text: nameField.text
                        }

                        Controls.Label {
                            Layout.fillWidth: true
                            elide: Text.ElideMiddle
                            color: Kirigami.Theme.disabledTextColor
                            font: Kirigami.Theme.smallFont
                            text: window.staged ? window.staged.source : ""
                        }
                    }
                }
            }
        }

        FormCard.FormHeader {
            title: qsTr("Details")
            visible: !window.installing
        }

        FormCard.FormCard {
            visible: !window.installing

            FormCard.FormTextFieldDelegate {
                id: nameField
                label: qsTr("Name")
                visible: !window.asInstaller
            }

            FormCard.FormDelegateSeparator { visible: !window.asInstaller }

            FormCard.FormTextFieldDelegate {
                id: descriptionField
                visible: !window.asInstaller
                label: qsTr("Description")
                placeholderText: qsTr("Shown in the application menu")
            }

            FormCard.FormDelegateSeparator { visible: !window.asInstaller }

            FormCard.FormComboBoxDelegate {
                id: categoryBox
                visible: !window.asInstaller
                text: qsTr("Category")
                description: qsTr("Where it lands in the application menu.")
                textRole: "label"
                valueRole: "value"
                model: [
                    { label: qsTr("Utility"), value: "Utility;" },
                    { label: qsTr("Game"), value: "Game;" },
                    { label: qsTr("Graphics"), value: "Graphics;" },
                    { label: qsTr("Office"), value: "Office;" },
                    { label: qsTr("Audio and video"), value: "AudioVideo;" },
                    { label: qsTr("Development"), value: "Development;" },
                    { label: qsTr("Network"), value: "Network;" },
                    { label: qsTr("System"), value: "System;" }
                ]
            }

            FormCard.FormDelegateSeparator { visible: !window.asInstaller }

            FormCard.FormTextFieldDelegate {
                id: gameIdField
                visible: !window.asInstaller

                property string store: "none"

                label: qsTr("Known as")
                placeholderText: qsTr("Leave empty if it is not a known title")
                onTextEdited: databaseHits.model = JSON.parse(Library.searchDatabase(text))
            }

            Repeater {
                id: databaseHits
                model: []

                FormCard.FormButtonDelegate {
                    required property var modelData

                    visible: !window.asInstaller
                    text: modelData.title
                    description: modelData.gameid + "  ·  " + modelData.store
                    onClicked: {
                        gameIdField.text = modelData.gameid
                        gameIdField.store = modelData.store
                        databaseHits.model = []
                    }
                }
            }

            FormCard.FormDelegateSeparator { }

            FormCard.FormComboBoxDelegate {
                id: kindBox
                text: qsTr("Type")
                description: window.staged && window.staged.kind === "installer"
                    ? qsTr("Detected as an installer")
                    : qsTr("Detected as a standalone program")
                model: [qsTr("Installer"), qsTr("Standalone program")]
                onActivated: window.asInstaller = currentIndex === 0
            }

            FormCard.FormDelegateSeparator { visible: window.asInstaller }

            FormCard.FormButtonDelegate {
                id: rootButton
                visible: window.asInstaller

                property string path: Library.prefixRoot

                icon.name: "folder-symbolic"
                text: qsTr("Install to")
                description: path + "/" + (nameField.text.length > 0 ? nameField.text : "…")
                onClicked: rootDialog.open()
            }

            FormCard.FormDelegateSeparator { }

            FormCard.FormSwitchDelegate {
                id: shortcutSwitch
                text: qsTr("Create a desktop entry")
                description: qsTr("Adds it to the application menu once it is ready.")
                checked: true
            }
        }

        FormCard.FormHeader {
            title: qsTr("Progress")
            visible: window.installing
        }

        FormCard.FormCard {
            visible: window.installing

            FormCard.AbstractFormDelegate {
                background: null
                contentItem: RowLayout {
                    spacing: Kirigami.Units.largeSpacing

                    Controls.BusyIndicator {
                        running: Library.busy
                        implicitWidth: Kirigami.Units.iconSizes.medium
                        implicitHeight: Kirigami.Units.iconSizes.medium
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
        }

        FormCard.FormCard {
            visible: window.installing
            Layout.topMargin: Kirigami.Units.largeSpacing

            Controls.ScrollView {
                Layout.fillWidth: true
                Layout.preferredHeight: Kirigami.Units.gridUnit * 14

                Controls.TextArea {
                    readOnly: true
                    wrapMode: TextEdit.NoWrap
                    font.family: "monospace"
                    text: Library.log
                    onTextChanged: cursorPosition = length
                }
            }
        }
    }
}
