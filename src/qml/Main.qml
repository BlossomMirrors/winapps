import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as Controls
import org.kde.kirigami as Kirigami
import org.kde.kirigamiaddons.formcard as FormCard

Kirigami.ApplicationWindow {
    id: root

    title: qsTr("Windows App Support")

    minimumWidth: Kirigami.Units.gridUnit * 20
    minimumHeight: Kirigami.Units.gridUnit * 20
    width: minimumWidth
    height: minimumHeight

    pageStack.initialPage: initPage
    globalDrawer: Kirigami.GlobalDrawer {
        isMenu: true
        actions: [
            Kirigami.Action {
                icon.name: "kde"
                text: "Open About page"
                onTriggered: pageStack.push(Qt.createComponent("org.kde.kirigamiaddons.formcard", "AboutPage"))
            }
        ]
    }

    //pageStack.globalToolBar.style: Kirigami.ApplicationHeaderStyle.None

    Component {
        id: initPage

        Kirigami.Page {
            title: qsTr("Windows App Support")

            ColumnLayout {
                RowLayout {
                    Layout.fillWidth: true

                    Controls.Button {
                        text: "Done"

                        onClicked: {
                            Qt.quit()
                        }
                    }
                }

                Controls.Label {
                    id: formattedText

                    textFormat: Text.RichText
                    wrapMode: Text.WordWrap
                    text: sourceArea.text

                    Layout.fillWidth: true
                    Layout.minimumHeight: Kirigami.Units.gridUnit * 5
                }
            }
        }
    }
}
