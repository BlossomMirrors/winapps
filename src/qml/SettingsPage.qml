import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as Controls
import QtQuick.Dialogs
import org.blossomos.winapps
import org.kde.kirigami as Kirigami
import org.kde.kirigamiaddons.formcard as FormCard

FormCard.FormCardPage {
    id: page

    title: qsTr("Settings")

    property var monitorModel: {
        const list = [{ label: qsTr("Auto (system primary)"), value: "" }]
        for (const screen of Qt.application.screens) {
            list.push({
                label: screen.model.length > 0 ? `${screen.name} — ${screen.model}` : screen.name,
                value: screen.name
            })
        }
        return list
    }

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
        FormCard.FormTextDelegate {
            text: qsTr("Installed build")
            description: Library.protonBuild.length > 0
                ? Library.protonBuild
                : qsTr("Not downloaded yet")
        }

        FormCard.FormDelegateSeparator { }

        FormCard.FormSwitchDelegate {
            text: qsTr("Keep it up to date")
            description: qsTr("Checks for a newer build before an app starts and installs it first.")
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
                : qsTr("Downloads the newest build right away.")
            enabled: !Library.busy && !Library.preparing
            onClicked: Library.ensureProton()
        }
    }

    FormCard.FormHeader {
        title: qsTr("Display")
    }

    FormCard.FormCard {
        FormCard.FormComboBoxDelegate {
            id: monitorBox
            text: qsTr("Primary monitor")
            description: qsTr("Which display Windows programs treat as the main one.")
            textRole: "label"
            valueRole: "value"
            model: page.monitorModel
            Component.onCompleted: {
                for (let i = 0; i < model.length; i++) {
                    if (model[i].value === Library.primaryMonitor) {
                        currentIndex = i
                        break
                    }
                }
            }
            onActivated: Library.primaryMonitor = currentValue
        }

        FormCard.FormDelegateSeparator { }

        FormCard.FormComboBoxDelegate {
            id: dpiBox
            text: qsTr("Display scaling (DPI)")
            description: qsTr("Overrides how large Windows programs render UI text and controls.")
            textRole: "label"
            valueRole: "value"
            model: [
                { label: qsTr("System default"), value: "" },
                { label: "100%", value: "96" },
                { label: "125%", value: "120" },
                { label: "150%", value: "144" },
                { label: "175%", value: "168" },
                { label: "200%", value: "192" }
            ]
            Component.onCompleted: {
                for (let i = 0; i < model.length; i++) {
                    if (model[i].value === Library.dpiOverride) {
                        currentIndex = i
                        break
                    }
                }
            }
            onActivated: Library.dpiOverride = currentValue
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
}
