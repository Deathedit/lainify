import QtQuick
import com.lainify
import QtQuick.Controls.Basic

// Labelled dropdown in the browser's `.field` style. `options` is `[{ value, label }]`.
Column {
    id: field

    property string label
    property string value
    property var options: []
    property string hint

    signal changed(string value)

    spacing: 6
    width: parent ? parent.width : 300

    Text {
        text: field.label
        font.family: Theme.font
        font.pixelSize: 12
        font.weight: Font.Medium
        color: Theme.zinc400
    }

    ComboBox {
        id: box

        width: field.width
        height: 36
        enabled: field.options.length > 0
        opacity: enabled ? 1 : 0.5
        model: field.options
        textRole: "label"
        valueRole: "value"
        currentIndex: field.options.findIndex(o => o.value === field.value)
        hoverEnabled: true
        focusPolicy: Qt.NoFocus
        onActivated: index => field.changed(field.options[index].value)

        background: Rectangle {
            radius: 8
            color: Theme.white(0.05)
            border.color: box.popup.visible ? Theme.accent400 : box.hovered ? Theme.white(0.20) : Theme.white(0.10)
            Behavior on border.color {
                ColorAnimation {
                    duration: 150
                }
            }
        }

        contentItem: Text {
            leftPadding: 12
            rightPadding: 36
            verticalAlignment: Text.AlignVCenter
            text: box.displayText
            elide: Text.ElideRight
            font.family: Theme.font
            font.pixelSize: 14
            color: Theme.zinc100
        }

        indicator: Icon {
            x: box.width - width - 10
            y: (box.height - height) / 2
            name: "chevron-down"
            size: 16
            color: Theme.zinc400
        }

        delegate: ItemDelegate {
            id: option

            required property int index
            required property var modelData

            width: ListView.view ? ListView.view.width : box.width
            height: 32
            hoverEnabled: true
            highlighted: box.highlightedIndex === index

            background: Rectangle {
                radius: 6
                color: option.highlighted ? Theme.white(0.08) : "transparent"
            }
            contentItem: Row {
                spacing: 8
                leftPadding: 4
                Icon {
                    anchors.verticalCenter: parent.verticalCenter
                    name: "check"
                    size: 14
                    color: Theme.accent300
                    opacity: box.currentIndex === option.index ? 1 : 0
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    width: option.width - 40
                    text: option.modelData.label
                    elide: Text.ElideRight
                    font.family: Theme.font
                    font.pixelSize: 14
                    color: box.currentIndex === option.index ? "white" : Theme.zinc200
                }
            }
        }

        popup: Popup {
            y: box.height + 4
            width: box.width
            height: Math.min(contentItem.implicitHeight + 8, 320)
            padding: 4

            contentItem: ListView {
                clip: true
                implicitHeight: contentHeight
                model: box.popup.visible ? box.delegateModel : null
                currentIndex: box.highlightedIndex
                boundsBehavior: Flickable.StopAtBounds
            }

            background: Glass {
                radius: 10
            }

            enter: Transition {
                NumberAnimation {
                    property: "opacity"
                    from: 0
                    to: 1
                    duration: 120
                }
            }
        }
    }

    Text {
        visible: field.hint !== ""
        width: field.width
        text: field.hint
        wrapMode: Text.Wrap
        lineHeight: 1.25
        font.family: Theme.font
        font.pixelSize: 12
        color: Theme.zinc500
    }
}
