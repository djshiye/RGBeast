import QtQuick
import QtQuick.Layouts

Sheet {
    id: ab
    preferredWidth: 400
    ColumnLayout {
        Layout.fillWidth: true
        spacing: 6
        Image {
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: 96
            Layout.preferredHeight: 96
            source: iconUrl
            sourceSize: Qt.size(192, 192)
            smooth: true
            mipmap: true
        }
        Text { Layout.alignment: Qt.AlignHCenter; text: "RGBeast"; color: Theme.text; font.pixelSize: 22; font.weight: Font.Bold }
        Text { Layout.alignment: Qt.AlignHCenter; text: "Version " + appVersion; color: Theme.text2; font.pixelSize: Theme.fsBody }
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
