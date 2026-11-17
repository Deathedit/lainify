pragma Singleton
import QtQuick

// The browser version's design tokens (Tailwind zinc + a violet accent), as sRGB.
QtObject {
    property string font: "Inter Variable"

    readonly property color zinc100: "#f4f4f5"
    readonly property color zinc200: "#e4e4e7"
    readonly property color zinc300: "#d4d4d8"
    readonly property color zinc400: "#a1a1aa"
    readonly property color zinc500: "#71717a"
    readonly property color zinc900: "#18181b"
    readonly property color zinc950: "#09090b"

    readonly property color accent300: "#cebfff"
    readonly property color accent400: "#baa3fe"
    readonly property color accent500: "#a17ff5"
    readonly property color rec: "#f53c41"

    readonly property color emerald400: "#34d399"
    readonly property color amber300: "#fcd34d"
    readonly property color amber400: "#fbbf24"
    readonly property color rose400: "#fb7185"
    readonly property color rose500: "#f43f5e"

    // No backdrop blur over video in Qt Quick, so the glass is a little more opaque than the
    // browser's 66% to stay readable over bright frames.
    readonly property color glass: Qt.rgba(19 / 255, 19 / 255, 23 / 255, 0.82)
    readonly property color glassBorder: Qt.rgba(1, 1, 1, 0.08)

    function white(alpha) {
        return Qt.rgba(1, 1, 1, alpha);
    }

    // cubic-bezier(0.22, 1, 0.36, 1), the browser's --ease-soft.
    readonly property var easeSoft: [0.22, 1, 0.36, 1, 1, 1]

    readonly property var fontFeatures: ({
            "cv11": 1,
            "ss01": 1
        })
    readonly property var tabular: ({
            "cv11": 1,
            "ss01": 1,
            "tnum": 1
        })

    /// `mm:ss`, or `h:mm:ss` once an hour has passed.
    function duration(seconds) {
        const total = Math.max(0, Math.floor(seconds));
        const h = Math.floor(total / 3600);
        const m = Math.floor(total % 3600 / 60);
        const s = total % 60;
        const pad = n => String(n).padStart(2, "0");
        return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${pad(m)}:${pad(s)}`;
    }

    /// 60 → "60", 59.9401 → "59.94".
    function fps(value) {
        return String(Math.round(value * 100) / 100);
    }
}
