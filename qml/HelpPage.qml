import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import OyNIx

// oynix://help — short answers to the questions people actually have,
// in the browser so they work offline. The website has the long version.
Rectangle {
    id: help
    property var win
    property string topic: "start"
    color: Theme.window

    readonly property var topics: [
        { id: "new", icon: "star", title: qsTr("What's new in 5.4"), body: qsTr(
"## What's new in 5.4\n\n" +
"**A fresh look.** The toolbar, tabs and favourites now float together on a translucent panel over a soft wash of your accent colour. The tab you're on is gently raised, the address field is set into the panel as a capsule, and cards have softer, rounder corners. Colours still come from your accent (Settings › Appearance), in light and dark.\n\n" +
"**Power extensions: GemSig.** `.poweroynix` packages can now be signed with GemSig (SHA3-512 and Ed25519, tied to the extension's name and version), and show a short **GemPrint** next to \"Verified by\" so you can compare two copies at a glance. Older signatures still work.\n\n" +
"**Under the hood.** Much of OyNIx now lives in StonedGem, a shared base for the next OyNIx: the address bar, password vault, content blocker, history and bookmarks, and extension packages. Nothing changes where your data is kept.\n\n" +
"**OyNIx Next (preview).** A separate preview of the next OyNIx — Servo engine, new window — runs alongside this one with its own profile. It's not a replacement yet.")
        },
        { id: "start", icon: "home", title: qsTr("Getting around"), body: qsTr(
"## Getting around\n\n" +
"**Tabs.** Ctrl+T opens one, Ctrl+W closes it, Ctrl+Shift+T brings back the last one you closed. Drag tabs to reorder them; right-click one to pin, mute or duplicate it.\n\n" +
"**Tab layouts.** Settings › Tabs offers tabs along the top, a compact single row, or **vertical tabs** in the sidebar. With **tree tabs** on, the tab you're on opens into a tree: the contents of a Markdown or PDF document (click to jump there) and the pages of the site you've moved through.\n\n" +
"**The address bar** takes addresses and searches. Type `!w rust` to search Wikipedia, `!gh` for GitHub; the full list is in Settings › Search.\n\n" +
"**The sidebar** (Ctrl+Shift+L) holds tabs, bookmarks, your reading list, history and Claude.\n\n" +
"**Every keyboard shortcut** is one keypress away: Ctrl+/.") },
        { id: "privacy", icon: "shield", title: qsTr("Privacy & blocking"), body: qsTr(
"## Privacy & blocking\n\n" +
"OyNIx blocks ads and trackers before they load, using the same filter lists as uBlock Origin. The shield in the toolbar shows what it stopped on the page you're on.\n\n" +
"**A site looks broken?** Click the shield and turn blocking off for that site. It stays off only there.\n\n" +
"**“Please turn off your ad blocker.”** The built-in **Wallbreaker** extension gets you past most of these walls. If one still appears, reload the page once.\n\n" +
"**Tracking parameters** (`utm_source`, `fbclid`…) are removed from addresses you open. Sign-in pages are never touched, so logging in always works.\n\n" +
"**Private windows** (Ctrl+Shift+N) keep no history, cookies or passwords, and extensions don't see them.") },
        { id: "passwords", icon: "key", title: qsTr("Passwords"), body: qsTr(
"## Passwords\n\n" +
"After you sign in, OyNIx offers to save the password. Next time, the form fills itself; if you have more than one account, pick another from the **key** in the address bar.\n\n" +
"Passwords are stored encrypted in your profile folder. Websites and extensions never see the list; OyNIx chooses what to fill from the tab's real address.\n\n" +
"**Moving from Firefox?** Settings › Passwords › Import reads passwords and bookmarks straight from your Firefox profile (Firefox can stay open). If you set a Primary Password in Firefox, export a CSV from `about:logins` instead and import that file.\n\n" +
"Show, copy or delete saved passwords in **Settings › Passwords**.") },
        { id: "bookmarks", icon: "star", title: qsTr("Bookmarks"), body: qsTr(
"## Bookmarks\n\n" +
"Press **Ctrl+D** or click the star to bookmark a page. A small editor opens: rename it, or choose a folder — the **favourites bar** under the address bar, Bookmarks, any folder you have, or a new one.\n\n" +
"**All Bookmarks**, at the right end of the favourites bar, lists every folder. The sidebar (Ctrl+B) can search them.\n\n" +
"Import from Firefox (Settings › Passwords › Import) or from any browser's HTML export (Settings › General).") },
        { id: "extensions", icon: "puzzle", title: qsTr("Extensions"), body: qsTr(
"## Extensions\n\n" +
"OyNIx runs **Chrome and Firefox extensions** (Manifest V2 and V3): open their store page and click Add. **OyNIx extensions** are smaller: a page of TypeScript and CSS you can read before installing, from the Extension Gallery.\n\n" +
"**Toolbar buttons** appear only for extensions you **pin** from the puzzle menu.\n\n" +
"**Widgets.** An extension with a `widget.tsx` adds a card to the Start Page.\n\n" +
"**Power extensions** (`.poweroynix`) can do more — watch and shape network traffic, restyle every shape in the browser, use the powerful extension APIs — so OyNIx installs them only when they carry a valid signature from a key it trusts. Wallbreaker is one; it ships with OyNIx.\n\n" +
"Writing your own: Settings › Extensions › Create, and the guide on the website.") },
        { id: "offline", icon: "sparkles", title: qsTr("Offline & games"), body: qsTr(
"## When you're offline\n\n" +
"If OyNIx can't reach the internet, the page turns into a game while you wait: **Go** against the computer on a 5×5, 7×7 or 9×9 board, or **Blackjack** against the dealer. OyNIx notices when the connection is back.\n\n" +
"Choose the game (or none) in **Settings › Misc**. They're also at `oynix://play`, and Ctrl+Alt+G opens them.") },
        { id: "updates", icon: "download", title: qsTr("Updates"), body: qsTr(
"## Updates\n\n" +
"OyNIx checks for a new version once a day and when it starts. A **red download icon** in the toolbar means one is waiting; click it to install, then restart. Your tabs come back.\n\n" +
"Installed from the **AUR** (`oynix-bin`)? Update with your AUR helper (`yay -Syu`); OyNIx tells you when there's something new.\n\n" +
"Settings › About shows your version and lets you check now.") },
        { id: "trouble", icon: "warning", title: qsTr("Troubleshooting"), body: qsTr(
"## Troubleshooting\n\n" +
"**OyNIx stops drawing on Wayland** (Hyprland and others). Some compositors stop telling a window when to draw its next frame. OyNIx now notices within about three seconds and brings the window back by itself. Each time it happens is noted in Settings › About › Freezes, which helps if you report it.\n\n" +
"**A sign-in ends on a broken page.** Go back and sign in again. OyNIx never rewrites sign-in addresses; if it keeps happening on one site, turn blocking off there (shield › this site).\n\n" +
"**A page won't load at all.** The error page says why in plain words; Details shows the technical reason.\n\n" +
"**Graphics problems.** Start OyNIx with `oynix --gpu gles`, `--gpu vulkan` or `--gpu software` to choose how it draws.\n\n" +
"**Still stuck?** Open an issue on GitHub with what you did, what happened, and your OyNIx version (Settings › About).") }
    ]
    readonly property var current: { for (var i = 0; i < topics.length; i++) if (topics[i].id === topic) return topics[i]; return topics[0] }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        // Topics
        Rectangle {
            Layout.preferredWidth: 240
            Layout.fillHeight: true
            color: Theme.surfaceContainerLow
            ColumnLayout {
                anchors.fill: parent; anchors.margins: 16
                spacing: 4
                Txt { text: qsTr("Help"); font.family: Theme.displayFont; font.pixelSize: 30; Layout.bottomMargin: 12; Layout.leftMargin: 8 }
                Repeater {
                    model: help.topics
                    delegate: AbstractButton {
                        id: tb
                        required property var modelData
                        Layout.fillWidth: true
                        implicitHeight: 40
                        hoverEnabled: true
                        readonly property bool sel: help.topic === modelData.id
                        background: Rectangle { radius: height / 2; color: tb.sel ? Theme.secondaryContainer : (tb.hovered ? Theme.hover : "transparent") }
                        contentItem: RowLayout {
                            spacing: 12
                            Icon { name: tb.modelData.icon; size: 18; color: tb.sel ? Theme.secondaryContainerFg : Theme.textDim; Layout.leftMargin: 14 }
                            Txt { text: tb.modelData.title; font.pixelSize: 14; font.weight: tb.sel ? Font.DemiBold : Font.Normal; Layout.fillWidth: true; color: tb.sel ? Theme.secondaryContainerFg : Theme.text }
                        }
                        onClicked: help.topic = modelData.id
                    }
                }
                Item { Layout.fillHeight: true }
                PushButton { text: qsTr("Keyboard Shortcuts"); Layout.fillWidth: true; onClicked: help.win.showShortcuts() }
                PushButton { text: qsTr("Full Documentation"); Layout.fillWidth: true; onClicked: help.win.openTab("https://trail-b1az3r.github.io/OyNIx/docs/", { afterCurrent: true }) }
            }
        }

        // Answer
        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentHeight: body.implicitHeight + 96
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: ScrollBar {}
            Text {
                id: body
                x: Math.max(32, (parent.width - width) / 2); y: 48
                width: Math.min(680, help.width - 240 - 64)
                text: help.current.body
                textFormat: Text.MarkdownText
                wrapMode: Text.WordWrap
                color: Theme.text
                linkColor: Theme.primary
                font.family: Theme.font
                font.pixelSize: 15
                lineHeight: 1.35
                onLinkActivated: function (link) { help.win.openTab(link, { afterCurrent: true }) }
            }
        }
    }
}
