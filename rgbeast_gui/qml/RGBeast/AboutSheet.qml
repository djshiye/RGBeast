import QtQuick
import QtQuick.Layouts

Sheet {
    id: ab
    preferredWidth: 400
    ColumnLayout {
        Layout.fillWidth: true
        spacing: 6
        Item {
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: 96
            Layout.preferredHeight: 96
            Glow { anchors.fill: parent; anchors.margins: 10; radius: 24; spread: 50; strength: Theme.dark ? 0.45 : 0.25 }
        Image {
            anchors.fill: parent
            source: iconUrl
            sourceSize: Qt.size(192, 192)
            smooth: true
            mipmap: true
        }
        }
        Text { Layout.alignment: Qt.AlignHCenter; Layout.topMargin: 6; text: "RGBeast"; color: Theme.text; font.pixelSize: 24; font.weight: Font.Bold; font.letterSpacing: -0.5 }
        Rectangle {
            Layout.alignment: Qt.AlignHCenter
            implicitWidth: ver.implicitWidth + 20
            implicitHeight: 22
            radius: 11
            color: Theme.fill
            border.width: 1
            border.color: Theme.hairline
            Text { id: ver; anchors.centerIn: parent; text: "v" + appVersion; color: Theme.text2; font.family: Theme.mono; font.pixelSize: Theme.fsCaption }
        }
        Text {
            Layout.fillWidth: true
            Layout.topMargin: 8
            text: "RGB lighting control for ASUS Aura motherboards, addressable fans, Kingston Fury memory and ASUS and Sapphire graphics cards. The app never touches hardware: a small sandboxed service does, behind polkit."
            color: Theme.text2
            font.pixelSize: Theme.fsBody
            lineHeight: 1.25
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.Wrap
        }
        Text {
            Layout.alignment: Qt.AlignHCenter
            Layout.topMargin: 6
            text: "<a href='https://github.com/djshiye/RGBeast'>Website</a>  ·  <a href='https://github.com/djshiye/RGBeast/issues'>Report an issue</a>  ·  MIT licence"
            textFormat: Text.StyledText
            linkColor: Theme.accentText
            color: Theme.text3
            font.pixelSize: Theme.fsCallout
            onLinkActivated: (link) => Qt.openUrlExternally(link)
            HoverHandler { cursorShape: parent.hoveredLink ? Qt.PointingHandCursor : Qt.ArrowCursor }
        }
    }
    buttons: [ Button { text: "Close"; onClicked: ab.close() } ]
}
