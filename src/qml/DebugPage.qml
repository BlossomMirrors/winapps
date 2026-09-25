import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as Controls
import org.blossomos.sangria
import org.kde.kirigami as Kirigami
import org.kde.kirigamiaddons.formcard as FormCard

FormCard.FormCardPage {
    id: page

    title: qsTr("Debugging")

    FormCard.FormHeader {
        title: qsTr("Logging")
    }

    FormCard.FormCard {
        FormCard.FormComboBoxDelegate {
            text: "UMU_LOG"
            description: qsTr("umu's own log level. debug also turns on PROTON_LOG.")
            model: ["", "1", "debug", "warn"]
            currentIndex: model.indexOf(Library.umuLog)
            onCurrentValueChanged: Library.umuLog = currentValue
        }

        FormCard.FormDelegateSeparator { }

        FormCard.FormComboBoxDelegate {
            text: "PROTON_VERB"
            description: qsTr("How Proton launches the executable.")
            model: ["waitforexitandrun", "run", "runinprefix", "getcompatpath", "getnativepath"]
            currentIndex: model.indexOf(Library.protonVerb)
            onCurrentValueChanged: Library.protonVerb = currentValue
        }
    }

    FormCard.FormHeader {
        title: qsTr("Runtime")
    }

    FormCard.FormCard {
        FormCard.FormTextFieldDelegate {
            label: "PROTONPATH"
            placeholderText: Library.defaultProton()
            description: qsTr("Leave empty to use the detected build shown above.")
            text: Library.protonPath
            onTextChanged: Library.protonPath = text
        }

        FormCard.FormDelegateSeparator { }

        FormCard.FormSwitchDelegate {
            text: "UMU_NO_RUNTIME"
            description: qsTr("Run outside the Steam Linux Runtime container.")
            checked: Library.noRuntime
            onToggled: Library.noRuntime = checked
        }

        FormCard.FormDelegateSeparator { }

        FormCard.FormSwitchDelegate {
            text: "UMU_RUNTIME_UPDATE=0"
            description: qsTr("Skip the runtime update check on launch.")
            checked: Library.skipRuntimeUpdate
            onToggled: Library.skipRuntimeUpdate = checked
        }

        FormCard.FormDelegateSeparator { }

        FormCard.FormSwitchDelegate {
            text: "UMU_NO_PROTON"
            description: qsTr("Use the runtime without Proton, for native binaries.")
            checked: Library.noProton
            onToggled: Library.noProton = checked
        }
    }

    FormCard.FormHeader {
        title: qsTr("Environment for the next run")
    }

    FormCard.FormCard {
        FormCard.FormTextDelegate {
            Layout.fillWidth: true
            textItem.wrapMode: Text.WordWrap
            text: Library.envPreview()
            description: qsTr("WINEPREFIX and GAMEID are filled in per app.")
        }
    }

    FormCard.FormHeader {
        title: qsTr("Output")

        actions: [
            Kirigami.Action {
                icon.name: "edit-clear-all-symbolic"
                text: qsTr("Clear")
                onTriggered: Library.clearLog()
            }
        ]
    }

    FormCard.FormCard {
        Controls.ScrollView {
            Layout.fillWidth: true
            Layout.preferredHeight: Kirigami.Units.gridUnit * 18

            Controls.TextArea {
                id: logView
                readOnly: true
                wrapMode: TextEdit.NoWrap
                font.family: "monospace"
                text: Library.log
                onTextChanged: cursorPosition = length
            }
        }
    }
}
