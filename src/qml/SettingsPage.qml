import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as Controls
import QtQuick.Dialogs
import org.blossomos.sangria
import org.kde.kirigami as Kirigami
import org.kde.kirigamiaddons.delegates as Delegates
import org.kde.kirigamiaddons.formcard as FormCard

FormCard.FormCardPage {
    id: page

    title: qsTr("Settings")

    readonly property var protonTools: JSON.parse(Library.protonTools())

    FolderDialog {
        id: prefixRootDialog
        title: qsTr("Where should new Windows prefixes live?")
        currentFolder: "file://" + Library.prefixRoot
        onAccepted: Library.prefixRoot = selectedFolder.toString().replace("file://", "")
    }

    FolderDialog {
        id: importPrefixDialog
        title: qsTr("Pick the Windows prefix to import")
        currentFolder: "file://" + Library.prefixRoot
        onAccepted: programPicker.load(selectedFolder)
    }

    Kirigami.Dialog {
        id: programPicker

        property var scan: null
        property var picked: []

        readonly property var apps: scan ? scan.apps : []
        readonly property int pickedCount: picked.filter(wanted => wanted).length

        function load(folder) {
            const found = JSON.parse(Library.scanPrefix(folder))
            picked = found ? found.apps.map(() => true) : []
            scan = found
            groupField.text = found ? found.group : ""
            open()
        }

        title: qsTr("Import programs")
        preferredWidth: Kirigami.Units.gridUnit * 28
        standardButtons: Kirigami.Dialog.Cancel
        onClosed: Library.discardScan()
        customFooterActions: [
            Kirigami.Action {
                icon.name: "list-add-symbolic"
                text: qsTr("Add to library")
                enabled: programPicker.pickedCount > 0
                onTriggered: {
                    const chosen = programPicker.apps.filter((app, index) => programPicker.picked[index])
                    Library.importPrefix(programPicker.scan.prefix, JSON.stringify(chosen),
                        groupField.text, desktopEntriesSwitch.checked)
                    programPicker.close()
                }
            }
        ]

        ColumnLayout {
            spacing: 0

            Controls.Label {
                Layout.fillWidth: true
                Layout.margins: Kirigami.Units.largeSpacing
                visible: programPicker.apps.length === 0
                wrapMode: Text.WordWrap
                text: programPicker.scan === null
                    ? qsTr("This folder is not a Windows prefix. Pick the folder that contains drive_c.")
                    : programPicker.scan.known > 0
                        ? qsTr("Every program in the Start Menu of this prefix is already in the library.")
                        : qsTr("The Start Menu of this prefix has no programs in it.")
            }

            Repeater {
                model: programPicker.apps

                FormCard.FormCheckDelegate {
                    required property var modelData
                    required property int index

                    text: modelData.name
                    description: modelData.description.length > 0
                        ? modelData.description
                        : modelData.exe
                    icon.name: modelData.icon.length > 0 ? "" : "application-x-ms-dos-executable"
                    icon.source: modelData.icon.length > 0 ? "file://" + modelData.icon : ""
                    icon.width: Kirigami.Units.iconSizes.medium
                    icon.height: Kirigami.Units.iconSizes.medium
                    checked: true
                    onToggled: {
                        const picked = programPicker.picked.slice()
                        picked[index] = checked
                        programPicker.picked = picked
                    }
                }
            }

            FormCard.FormDelegateSeparator {
                visible: groupField.visible
            }

            FormCard.FormTextFieldDelegate {
                id: groupField
                visible: programPicker.pickedCount + (programPicker.scan ? programPicker.scan.known : 0) > 1
                label: qsTr("Group name")
                description: qsTr("Programs from this prefix share one card in the library.")
            }

            FormCard.FormDelegateSeparator {
                visible: programPicker.apps.length > 0
            }

            FormCard.FormSwitchDelegate {
                id: desktopEntriesSwitch
                visible: programPicker.apps.length > 0
                text: qsTr("Create desktop entries")
                description: qsTr("Adds them to the application menu.")
                checked: true
            }
        }
    }

    FormCard.FormHeader {
        title: qsTr("Compatibility")
    }

    FormCard.FormCard {
        FormCard.FormComboBoxDelegate {
            id: toolBox
            text: qsTr("Compatibility tool")
            description: qsTr("What runs Windows programs. Applies the next time a program starts.")
            textRole: "label"
            valueRole: "value"
            model: page.protonTools
            comboBoxDelegate: Delegates.RoundedItemDelegate {
                id: toolItem
                required property var model
                required property int index

                implicitWidth: ListView.view ? ListView.view.width : Kirigami.Units.gridUnit * 16
                text: model.label
                highlighted: toolBox.highlightedIndex === index
                contentItem: ToolRow { item: toolItem }
            }
            dialogDelegate: Delegates.RoundedItemDelegate {
                id: toolDialogItem
                required property var model
                required property int index

                implicitWidth: ListView.view ? ListView.view.width : Kirigami.Units.gridUnit * 16
                text: model.label
                contentItem: ToolRow { item: toolDialogItem }
                onClicked: {
                    toolBox.currentIndex = index
                    toolBox.activated(index)
                    toolBox.closeDialog()
                }
            }
            Component.onCompleted: {
                for (let i = 0; i < model.length; i++) {
                    if (model[i].value === Library.protonTool) {
                        currentIndex = i
                        break
                    }
                }
            }
            onActivated: Library.protonTool = currentValue
        }

        FormCard.FormDelegateSeparator { }

        FormCard.FormTextDelegate {
            text: qsTr("Proton-Wineland build")
            description: Library.protonBuild.length > 0
                ? Library.protonBuild
                : qsTr("Not downloaded yet")
        }

        FormCard.FormDelegateSeparator { }

        FormCard.FormSwitchDelegate {
            text: qsTr("Keep it up to date")
            description: qsTr("Checks for a newer Proton-Wineland before an app starts and installs it first.")
            enabled: Library.protonTool.length === 0
            checked: Library.autoUpdate
            onToggled: Library.autoUpdate = checked
        }

        FormCard.FormDelegateSeparator { }

        FormCard.FormSwitchDelegate {
            text: qsTr("Run on Wayland")
            description: qsTr("Native Wayland output instead of XWayland. Turn it off if a program renders wrong or refuses to start.")
            checked: Library.wayland
            onToggled: Library.wayland = checked
        }

        FormCard.FormDelegateSeparator { }

        FormCard.FormSwitchDelegate {
            text: qsTr("Match the desktop theme")
            description: qsTr("Applies the desktop's light or dark colours and accent colour to Windows programs and Wine's own windows.")
            checked: Library.followTheme
            onToggled: Library.followTheme = checked
        }

        FormCard.FormDelegateSeparator { }

        FormCard.FormButtonDelegate {
            icon.name: "cloud-download-symbolic"
            text: qsTr("Check now")
            description: Library.preparing && Library.status.length > 0
                ? Library.status
                : qsTr("Downloads the newest Proton-Wineland right away.")
            enabled: !Library.busy && !Library.preparing
            onClicked: Library.ensureProton()
        }
    }

    FormCard.FormHeader {
        title: qsTr("Storage")
    }

    FormCard.FormCard {
        FormCard.FormButtonDelegate {
            icon.name: "folder-symbolic"
            text: qsTr("Default install location")
            description: Library.prefixRoot
            onClicked: prefixRootDialog.open()
        }

        FormCard.FormDelegateSeparator { }

        FormCard.FormTextDelegate {
            text: qsTr("Prefixes can grow to several GB each. New installs suggest this folder, and it can still be changed per app.")
        }

        FormCard.FormDelegateSeparator { }

        FormCard.FormButtonDelegate {
            icon.name: "document-import-symbolic"
            text: qsTr("Import a prefix…")
            description: qsTr("Adds the programs from the Start Menu of an existing Windows prefix. A suite like Office shows up as Word, Excel and the rest.")
            onClicked: importPrefixDialog.open()
        }
    }

    component ToolRow: RowLayout {
        id: row

        required property var item

        spacing: Kirigami.Units.smallSpacing

        Delegates.DefaultContentItem {
            itemDelegate: row.item
            Layout.fillWidth: true
        }

        Kirigami.Badge {
            visible: row.item.model.recommended
            text: qsTr("Recommended")
            type: Kirigami.Badge.Type.Positive
        }
    }
}
