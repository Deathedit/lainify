import QtQuick
import com.lainify

Rectangle {
    property alias text: label.text

    implicitWidth: Math.max(20, label.implicitWidth + 12)
    implicitHeight: 20
    radius: 6
    color: Theme.white(0.05)
    border.color: Theme.white(0.10)

    Text {
        id: label
        anchors.centerIn: parent
        font.family: Theme.font
        font.pixelSize: 11
        font.weight: Font.Medium
        color: Theme.zinc300
    }
}
