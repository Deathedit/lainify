import QtQuick
import com.lainify

// Short messages at the top centre; click to dismiss. At most three at a time.
Column {
    id: toasts

    property int nextId: 0

    function show(message, tone, duration) {
        items.append({
            "toastId": ++nextId,
            "message": message,
            "tone": tone || "info",
            "duration": duration || 3500
        });
        while (items.count > 3)
            items.remove(0);
    }

    function dismiss(id) {
        for (let i = 0; i < items.count; ++i) {
            if (items.get(i).toastId === id) {
                items.remove(i);
                return;
            }
        }
    }

    anchors.horizontalCenter: parent.horizontalCenter
    y: 16
    z: 50
    spacing: 8

    ListModel {
        id: items
    }

    add: Transition {
        NumberAnimation {
            property: "opacity"
            from: 0
            to: 1
            duration: 200
        }
        NumberAnimation {
            property: "y"
            from: -8
            duration: 200
            easing.type: Easing.OutCubic
        }
    }
    move: Transition {
        NumberAnimation {
            property: "y"
            duration: 200
        }
    }

    Repeater {
        model: items

        Glass {
            id: toast

            required property int toastId
            required property string message
            required property string tone
            required property int duration

            anchors.horizontalCenter: parent.horizontalCenter
            width: Math.min(row.implicitWidth + 28, toasts.parent.width - 32)
            height: 36
            radius: 18

            Row {
                id: row
                anchors.verticalCenter: parent.verticalCenter
                x: 12
                spacing: 10

                Icon {
                    anchors.verticalCenter: parent.verticalCenter
                    size: 16
                    name: toast.tone === "success" ? "circle-check" : toast.tone === "error" ? "circle-alert" : "info"
                    color: toast.tone === "success" ? Theme.emerald400 : toast.tone === "error" ? Theme.rose400 : Theme.accent300
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    width: Math.min(implicitWidth, toasts.parent.width - 90)
                    elide: Text.ElideRight
                    text: toast.message
                    font.family: Theme.font
                    font.pixelSize: 14
                    color: Theme.zinc100
                }
            }

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: toasts.dismiss(toast.toastId)
            }

            Timer {
                running: true
                interval: toast.duration
                onTriggered: toasts.dismiss(toast.toastId)
            }
        }
    }
}
