import QtQuick
import com.lainify

// A Lucide icon drawn in any colour: the SVG is assembled with the colour baked in.
Image {
    id: icon

    property string name
    property color color: Theme.zinc300
    property real size: 18
    property real strokeWidth: 2
    property bool filled: false

    readonly property string hex: {
        const c = icon.color;
        const h = v => Math.round(v * 255).toString(16).padStart(2, "0");
        return "#" + h(c.r) + h(c.g) + h(c.b);
    }

    width: size
    height: size
    opacity: color.a
    sourceSize: Qt.size(Math.ceil(size * Screen.devicePixelRatio), Math.ceil(size * Screen.devicePixelRatio))
    smooth: true
    source: name && Icons.paths[name]
        ? "data:image/svg+xml;utf8," + encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="${filled ? hex : "none"}" stroke="${hex}" stroke-width="${strokeWidth}" stroke-linecap="round" stroke-linejoin="round">${Icons.paths[name]}</svg>`)
        : ""
}
