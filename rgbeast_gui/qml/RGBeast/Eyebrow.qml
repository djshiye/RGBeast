import QtQuick

// A small uppercase label with wide tracking: section names and card titles.
Text {
    color: Theme.text3
    font.pixelSize: Theme.fsCaption
    font.weight: Font.Bold
    font.letterSpacing: Theme.eyebrowTracking
    font.capitalization: Font.AllUppercase
    elide: Text.ElideRight
}
