import QtQuick
import com.lainify

// -60 dBFS … 0 dBFS: green up to -15, amber to -6, red above.
Column {
    id: meter

    property real levelLeft: 0
    property real levelRight: 0
    property bool active: true

    width: 56
    spacing: 3
    opacity: active ? 1 : 0.3

    Behavior on opacity {
        NumberAnimation {
            duration: 150
        }
    }

    Repeater {
        model: [meter.levelLeft, meter.levelRight]

        Rectangle {
            required property real modelData

            width: meter.width
            height: 3
            radius: 1.5
            color: Theme.white(0.10)
            clip: true

            Item {
                width: parent.width * Math.max(0, Math.min(1, modelData))
                height: parent.height
                clip: true

                Rectangle {
                    width: meter.width
                    height: 3
                    radius: 1.5
                    gradient: Gradient {
                        orientation: Gradient.Horizontal
                        GradientStop {
                            position: 0
                            color: Theme.emerald400
                        }
                        GradientStop {
                            position: 0.72
                            color: Theme.emerald400
                        }
                        GradientStop {
                            position: 0.86
                            color: Theme.amber300
                        }
                        GradientStop {
                            position: 1
                            color: Theme.rose500
                        }
                    }
                }
            }
        }
    }
}
