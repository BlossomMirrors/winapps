import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as Controls
import QtQuick.Dialogs
import org.blossomos.winapps
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
