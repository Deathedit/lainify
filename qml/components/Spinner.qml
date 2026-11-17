import QtQuick
import com.lainify

Icon {
    name: "loader-circle"

    RotationAnimator on rotation {
        from: 0
        to: 360
        duration: 900
        loops: Animation.Infinite
        running: visible
    }
}
