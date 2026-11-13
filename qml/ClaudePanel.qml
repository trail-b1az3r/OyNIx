import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import OyNIx

// Sidebar panel for Claude for OyNIx: connection setup, chat (API key) and
// the activity log of everything Claude did in the browser.
Item {
    id: panel
    property var win
    readonly property var agent: win.app.agent
    readonly property var bridge: win.app.agentBridge
    property string view: "chat"      // chat | activity | setup
    property var keyStatus: { Core.settingsJson; return JSON.parse(bridge.apiKeyStatus()) }

    function copy(t) { win.copyText(t) }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 10

        // Header
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            MShape { shape: "cookie9"; size: 28; color: Theme.primaryContainer
                Icon { anchors.centerIn: parent; name: "sparkles"; size: 16; color: Theme.primaryContainerFg } }
            ColumnLayout {
                spacing: 0
                Layout.fillWidth: true
                Txt { text: "Claude"; font.pixelSize: 16; font.weight: Font.DemiBold }
                Txt {
                    font.pixelSize: 11; color: Theme.textDim
                    text: !Core.agentEnabled ? qsTr("Off") :
                          panel.agent.stopped ? qsTr("Stopped") :
                          panel.agent.working ? qsTr("Working…") :
                          panel.bridge.connections > 0 ? qsTr("Connected to Claude app") : qsTr("Ready")
                }
            }
            IconButton { glyph: "plus"; tip: qsTr("New chat"); visible: panel.view === "chat"; onClicked: panel.agent.newChat() }
            IconButton { glyph: "list"; tip: qsTr("Activity"); checked: panel.view === "activity"; onClicked: panel.view = panel.view === "activity" ? "chat" : "activity" }
            IconButton { glyph: "sliders"; tip: qsTr("Setup"); checked: panel.view === "setup"; onClicked: panel.view = panel.view === "setup" ? "chat" : "setup" }
        }

        // Turned off
        ColumnLayout {
            visible: !Core.agentEnabled
            Layout.fillWidth: true
            spacing: 10
            Txt {
                Layout.fillWidth: true; wrapMode: Text.WordWrap; elide: Text.ElideNone; color: Theme.textDim
                text: qsTr("Let Claude see and control OyNIx: read pages, click, type, navigate and manage tabs. You choose which sites it may use, and you can stop it at any time.")
            }
            PushButton { text: qsTr("Turn On Claude for OyNIx"); primary: true; onClicked: Core.setSetting("agentEnabled", "true") }
        }

        // ── Setup ──
        Flickable {
            visible: Core.agentEnabled && panel.view === "setup"
            Layout.fillWidth: true; Layout.fillHeight: true
            contentHeight: setupCol.implicitHeight
            clip: true
            ColumnLayout {
                id: setupCol
                width: parent.width
                spacing: 12
                MCard {
                    Layout.fillWidth: true
                    title: qsTr("Use your Claude account")
                    Txt { Layout.fillWidth: true; wrapMode: Text.WordWrap; elide: Text.ElideNone; color: Theme.textDim; font.pixelSize: 12
                        text: qsTr("Claude Code and Claude Desktop, signed in with your Claude plan, can control OyNIx through MCP. Add OyNIx once:") }
                    Txt { text: qsTr("Claude Code"); font.weight: Font.DemiBold; font.pixelSize: 12 }
                    CodeBox { Layout.fillWidth: true; text: panel.bridge.mcpCommand(); onCopy: panel.copy(text) }
                    Txt { text: qsTr("Claude Desktop (claude_desktop_config.json)"); font.weight: Font.DemiBold; font.pixelSize: 12 }
                    CodeBox { Layout.fillWidth: true; text: panel.bridge.desktopConfig(); onCopy: panel.copy(text) }
                }
                MCard {
                    Layout.fillWidth: true
                    title: qsTr("Chat here with an API key")
                    Txt { Layout.fillWidth: true; wrapMode: Text.WordWrap; elide: Text.ElideNone; color: Theme.textDim; font.pixelSize: 12
                        text: panel.keyStatus.configured ? qsTr("Using key %1 (%2).").arg(panel.keyStatus.hint).arg(panel.keyStatus.source)
                                                         : qsTr("The panel calls the Claude API directly. Usage is billed to your Anthropic API account.") }
                    RowLayout {
                        Layout.fillWidth: true
                        MTextField { id: keyField; Layout.fillWidth: true; placeholder: "sk-ant-…"; echoMode: TextInput.Password }
                        PushButton { text: qsTr("Save"); primary: true; enabled: keyField.text.trim().length > 20
                            onClicked: { panel.bridge.setApiKey(keyField.text); keyField.text = ""; panel.keyStatus = JSON.parse(panel.bridge.apiKeyStatus()) } }
                    }
                    PushButton { visible: panel.keyStatus.source === "settings"; text: qsTr("Remove Key"); destructive: true
                        onClicked: { panel.bridge.setApiKey(""); panel.keyStatus = JSON.parse(panel.bridge.apiKeyStatus()) } }
                    Txt { text: qsTr("Model"); font.weight: Font.DemiBold; font.pixelSize: 12 }
                    Segmented {
                        Layout.fillWidth: true
                        options: panel.agent.models.map(function (m) { return { v: m.id, t: m.name.replace("Claude ", "") } })
                        value: JSON.parse(Core.settingsJson).agentModel
                        onPicked: function (v) { Core.setSetting("agentModel", JSON.stringify(v)) }
                    }
                }
                PushButton { text: qsTr("Permissions & blocked sites…"); onClicked: panel.win.app.openSettings("extensions") }
            }
        }

        // ── Activity ──
        ListView {
            visible: Core.agentEnabled && panel.view === "activity"
            Layout.fillWidth: true; Layout.fillHeight: true
            clip: true
            spacing: 4
            model: panel.agent.activity.slice().reverse()
            delegate: RowLayout {
                required property var modelData
                width: ListView.view.width
                spacing: 8
                Icon { name: modelData.ok ? "check" : "warning"; size: 14; color: modelData.ok ? Theme.primary : Theme.danger }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0
                    Txt { Layout.fillWidth: true; text: modelData.summary; font.pixelSize: 12; wrapMode: Text.WordWrap; elide: Text.ElideNone; maximumLineCount: 3 }
                    Txt { Layout.fillWidth: true; font.pixelSize: 10; color: Theme.textFaint
                        text: new Date(modelData.time).toLocaleTimeString(Qt.locale(), "HH:mm:ss") + " · " + (modelData.source === "mcp" ? qsTr("Claude app") : qsTr("panel")) + (modelData.host ? " · " + modelData.host : "") }
                }
            }
            Txt { anchors.centerIn: parent; visible: parent.count === 0; text: qsTr("No activity yet"); color: Theme.textFaint }
        }

        // ── Chat ──
        ListView {
            id: chatList
            visible: Core.agentEnabled && panel.view === "chat"
            Layout.fillWidth: true; Layout.fillHeight: true
            clip: true
            spacing: 8
            model: panel.agent.chat
            onCountChanged: Qt.callLater(positionViewAtEnd)
            delegate: Item {
                required property var modelData
                width: chatList.width
                height: bubble.height
                Rectangle {
                    id: bubble
                    readonly property bool mine: modelData.kind === "user"
                    readonly property bool toolRow: modelData.kind === "tool"
                    x: mine ? parent.width - width : 0
                    width: toolRow ? parent.width : Math.min(parent.width * (mine ? 0.85 : 1), msg.implicitWidth + 24)
                    height: toolRow ? 30 : msg.implicitHeight + 18
                    radius: toolRow ? 15 : 18
                    color: mine ? Theme.primaryContainer : toolRow ? Theme.surfaceContainerHigh : modelData.kind === "note" ? "transparent" : Theme.surfaceContainerLow
                    Text {
                        id: msg
                        visible: !bubble.toolRow
                        x: 12; y: 9
                        width: Math.min(implicitWidth, chatList.width * (bubble.mine ? 0.85 : 1) - 24)
                        text: modelData.text || ""
                        textFormat: modelData.kind === "assistant" ? Text.MarkdownText : Text.PlainText
                        wrapMode: Text.Wrap
                        color: bubble.mine ? Theme.primaryContainerFg : modelData.kind === "note" ? Theme.textDim : Theme.text
                        font.family: Theme.font; font.pixelSize: 13
                        font.italic: modelData.kind === "note"
                        onLinkActivated: function (l) { panel.win.openTab(l, { afterCurrent: true }) }
                    }
                    RowLayout {
                        visible: bubble.toolRow
                        anchors.fill: parent; anchors.leftMargin: 10; anchors.rightMargin: 10
                        spacing: 8
                        LoadingShape { visible: modelData.status === "running"; size: 14 }
                        Icon { visible: modelData.status !== "running"; name: modelData.status === "error" ? "warning" : "check"; size: 13
                            color: modelData.status === "error" ? Theme.danger : Theme.primary }
                        Txt { Layout.fillWidth: true; text: modelData.text + (modelData.detail ? " — " + modelData.detail : ""); font.pixelSize: 12; color: Theme.textDim }
                    }
                }
            }
            ColumnLayout {
                anchors.centerIn: parent
                width: parent.width - 20
                visible: chatList.count === 0
                spacing: 8
                MShape { Layout.alignment: Qt.AlignHCenter; shape: "flower"; size: 56; color: Theme.secondaryContainer
                    Icon { anchors.centerIn: parent; name: "sparkles"; size: 26; color: Theme.secondaryContainerFg } }
                Txt { Layout.alignment: Qt.AlignHCenter; text: qsTr("What should Claude do?"); font.pixelSize: 15; font.weight: Font.DemiBold }
                Txt { Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap; elide: Text.ElideNone; color: Theme.textDim; font.pixelSize: 12
                    text: panel.keyStatus.configured ? qsTr("For example: “Summarise this page”, “Find a train to Oslo tomorrow morning”, or “Fill in this form with my shipping address”.")
                                                     : qsTr("Add an API key or connect the Claude app in Setup.") }
            }
        }

        Txt {
            visible: panel.view === "chat" && panel.agent.chatError !== ""
            Layout.fillWidth: true; wrapMode: Text.WordWrap; elide: Text.ElideNone
            text: panel.agent.chatError; color: Theme.danger; font.pixelSize: 12
        }

        // Composer
        Rectangle {
            visible: Core.agentEnabled && panel.view === "chat"
            Layout.fillWidth: true
            implicitHeight: Math.min(160, Math.max(52, input.contentHeight + 22))
            radius: 26
            color: Theme.surfaceContainerHighest
            RowLayout {
                anchors.fill: parent; anchors.leftMargin: 16; anchors.rightMargin: 6
                Flickable {
                    Layout.fillWidth: true; Layout.fillHeight: true
                    contentHeight: input.contentHeight
                    clip: true
                    TextEdit {
                        id: input
                        width: parent.width
                        y: Math.max(0, (parent.height - contentHeight) / 2)
                        wrapMode: TextEdit.Wrap
                        color: Theme.text; font.family: Theme.font; font.pixelSize: 14
                        selectionColor: Theme.selection
                        Keys.onReturnPressed: function (e) {
                            if (e.modifiers & Qt.ShiftModifier) { e.accepted = false; return }
                            panel.agent.sendChat(text); text = ""
                        }
                        Txt { visible: !input.text; text: qsTr("Ask Claude to do something…"); color: Theme.textFaint; font.pixelSize: 14 }
                    }
                }
                IconButton {
                    implicitWidth: 40; implicitHeight: 40; round: true
                    glyph: panel.agent.chatBusy ? "close" : "send"
                    filledIcon: false
                    tonal: true
                    tip: panel.agent.chatBusy ? qsTr("Stop") : qsTr("Send")
                    onClicked: { if (panel.agent.chatBusy) panel.agent.stop(); else { panel.agent.sendChat(input.text); input.text = "" } }
                }
            }
        }
    }

    // Small helpers ---------------------------------------------------
    component CodeBox: Rectangle {
        id: cb
        property string text
        signal copy()
        implicitHeight: code.implicitHeight + 20
        radius: 12
        color: Theme.surfaceContainerHighest
        Text {
            id: code
            anchors.left: parent.left; anchors.right: copyBtn.left; anchors.top: parent.top; anchors.margins: 10
            text: cb.text; wrapMode: Text.WrapAnywhere
            font.family: "monospace"; font.pixelSize: 11; color: Theme.text
        }
        IconButton { id: copyBtn; anchors.right: parent.right; anchors.top: parent.top; anchors.margins: 4; glyph: "copy"; tip: qsTr("Copy"); onClicked: cb.copy() }
    }
}
