import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import QtQuick.Window
import QtQuick.Dialogs
import OyNIx

// Settings UI with a navigation rail. Shown in a browser tab
// (oynix://settings/<page>) or inside SettingsWindow.
Rectangle {
    id: sw
    property var app
    property string page: "general"
    property bool shown: visible
    color: Theme.surface

    function set(key, value) { Core.setSetting(key, JSON.stringify(value)) }
    readonly property var s: JSON.parse(Core.settingsJson)

    readonly property var pages: [
        { id: "general", icon: "sliders", title: qsTr("General") },
        { id: "appearance", icon: "sun", title: qsTr("Appearance") },
        { id: "tabs", icon: "tabs", title: qsTr("Tabs") },
        { id: "search", icon: "search", title: qsTr("Search") },
        { id: "privacy", icon: "shield", title: qsTr("Privacy") },
        { id: "websites", icon: "globe", title: qsTr("Websites") },
        { id: "extensions", icon: "puzzle", title: qsTr("Extensions") },
        { id: "profiles", icon: "user", title: qsTr("Profiles") },
        { id: "advanced", icon: "code", title: qsTr("Advanced") },
        { id: "about", icon: "star", title: qsTr("About") }
    ]


    // Material 3 navigation rail.
    Rectangle {
        id: rail
        width: 96
        anchors.top: parent.top; anchors.bottom: parent.bottom; anchors.left: parent.left
        color: Theme.surfaceContainer
        Flickable {
            anchors.fill: parent
            contentHeight: railCol.implicitHeight + 24
            clip: true
            Column {
                id: railCol
                y: 16
                width: parent.width
                spacing: 4
                Repeater {
                    model: sw.pages
                    delegate: AbstractButton {
                        id: railItem
                        required property var modelData
                        readonly property bool sel: sw.page === modelData.id
                        width: rail.width; height: 64
                        hoverEnabled: true
                        onClicked: sw.page = modelData.id
                        contentItem: Column {
                            spacing: 4
                            Rectangle {
                                anchors.horizontalCenter: parent.horizontalCenter
                                width: railItem.sel ? 56 : (railItem.hovered ? 56 : 32); height: 32; radius: 16
                                color: railItem.sel ? Theme.secondaryContainer : (railItem.hovered ? Theme.hover : "transparent")
                                Behavior on width { SpringAnimation { spring: Theme.springDefault; damping: Theme.damping } }
                                Icon { anchors.centerIn: parent; name: railItem.modelData.icon; size: 20
                                       color: railItem.sel ? Theme.secondaryContainerFg : Theme.surfaceVariantFg }
                            }
                            Txt { anchors.horizontalCenter: parent.horizontalCenter; text: railItem.modelData.title; font.pixelSize: 12
                                  font.weight: railItem.sel ? Font.DemiBold : Font.Medium
                                  color: railItem.sel ? Theme.surfaceFg : Theme.surfaceVariantFg }
                        }
                    }
                }
            }
        }
    }

    // ── Reusable rows ────────────────────────────────────────────────
    component Section: ColumnLayout {
        property string title: ""
        default property alias rows: body.data
        Layout.fillWidth: true
        spacing: 6
        Txt { text: parent.title; font.weight: Font.DemiBold; color: Theme.primary; font.pixelSize: 14; visible: text !== ""; Layout.leftMargin: 8 }
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: body.implicitHeight + 16
            radius: Theme.shapeXL - 4
            color: Theme.surfaceContainerLow
            ColumnLayout { id: body; anchors.fill: parent; anchors.margins: 4; spacing: 0 }
        }
    }
    component Row_: RowLayout {
        property string label: ""
        property string hint: ""
        Layout.fillWidth: true
        Layout.minimumHeight: hint ? 50 : 38
        Layout.leftMargin: 12
        Layout.rightMargin: 12
        spacing: 12
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 1
            Txt { text: parent.parent.label; Layout.fillWidth: true }
            Txt { visible: text !== ""; text: parent.parent.hint; color: Theme.textDim; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true; elide: Text.ElideNone }
        }
    }
    component SwitchRow: Row_ {
        id: sr
        property string key: ""
        Toggle { checked: sw.s[sr.key] === true; onToggled: sw.set(sr.key, checked) }
    }
    component ChoiceRow: Row_ {
        id: cr
        property string key: ""
        property var options: []   // [{v, t}]
        ComboBox {
            id: combo
            Layout.preferredWidth: 220
            implicitHeight: 28
            model: cr.options
            textRole: "t"
            valueRole: "v"
            currentIndex: { for (var i = 0; i < cr.options.length; i++) if (cr.options[i].v === sw.s[cr.key]) return i; return 0 }
            onActivated: function (i) { sw.set(cr.key, cr.options[i].v) }
            font.pixelSize: 12
            background: Rectangle { radius: 6; color: Theme.field; border.color: Theme.separator }
            contentItem: Txt { leftPadding: 10; text: combo.displayText; font.pixelSize: 12 }
            popup.background: Rectangle { radius: 8; color: Theme.popover; border.color: Theme.separator }
            delegate: ItemDelegate {
                required property var modelData
                required property int index
                width: combo.width
                height: 28
                highlighted: combo.highlightedIndex === index
                background: Rectangle { radius: 5; color: parent.highlighted ? Theme.accent : "transparent" }
                contentItem: Txt { text: modelData.t; color: parent.highlighted ? "white" : Theme.text; font.pixelSize: 12 }
            }
        }
    }
    component TextRow: Row_ {
        id: tr
        property string key: ""
        property string placeholder: ""
        Rectangle {
            Layout.preferredWidth: 260; implicitHeight: 28; radius: 6; color: Theme.field; border.color: Theme.separator
            TextInput {
                id: ti
                anchors.fill: parent; anchors.leftMargin: 8; anchors.rightMargin: 8
                verticalAlignment: TextInput.AlignVCenter
                color: Theme.text; font.family: Theme.font; font.pixelSize: 12; clip: true
                text: sw.s[tr.key] || ""
                onEditingFinished: if (text !== (sw.s[tr.key] || "")) sw.set(tr.key, text)
                Txt { visible: !ti.text; text: tr.placeholder; color: Theme.textFaint; anchors.verticalCenter: parent.verticalCenter; font.pixelSize: 12 }
            }
        }
    }
    component SiteList: ColumnLayout {
        id: sl
        property string key
        property string empty
        property bool canAdd: false
        readonly property var items: sw.s[key] || []
        Layout.fillWidth: true
        spacing: 6
        Flow {
            Layout.fillWidth: true
            spacing: 8
            Repeater {
                model: sl.items
                delegate: Rectangle {
                    required property string modelData
                    required property int index
                    height: 32; width: chipText.implicitWidth + 44; radius: Theme.shapeS
                    color: "transparent"; border.color: Theme.outline; border.width: 1
                    Txt { id: chipText; x: 12; anchors.verticalCenter: parent.verticalCenter; text: modelData; font.pixelSize: 13 }
                    IconButton { anchors.right: parent.right; anchors.verticalCenter: parent.verticalCenter; implicitWidth: 28; implicitHeight: 28; glyph: "close"; iconSize: 14
                        onClicked: { var l = sl.items.slice(); l.splice(index, 1); sw.set(sl.key, l) } }
                }
            }
        }
        Txt { visible: sl.items.length === 0; text: sl.empty; color: Theme.textFaint; font.pixelSize: 12 }
        RowLayout {
            visible: sl.canAdd
            MTextField { id: siteInput; Layout.preferredWidth: 240; placeholder: "example.com"; onAccepted: addBtn.clicked() }
            PushButton { id: addBtn; text: qsTr("Add"); enabled: siteInput.text.trim().indexOf(".") > 0
                onClicked: { var l = sl.items.slice(); var h = siteInput.text.trim().toLowerCase().replace(/^https?:\/\//, "").split("/")[0]; if (l.indexOf(h) < 0) l.push(h); sw.set(sl.key, l); siteInput.text = "" } }
        }
    }
    component Divider: Rectangle { Layout.fillWidth: true; Layout.leftMargin: 12; Layout.rightMargin: 12; height: 1; color: Theme.separator }

    // ── Pages ────────────────────────────────────────────────────────
    ScrollView {
        id: scroll
        anchors.top: parent.top; anchors.bottom: parent.bottom; anchors.right: parent.right; anchors.left: rail.right
        contentWidth: availableWidth
        clip: true

        Loader {
            width: scroll.availableWidth
            sourceComponent: ({
                general: generalPage, appearance: appearancePage, tabs: tabsPage, search: searchPage,
                privacy: privacyPage, websites: websitesPage, extensions: extensionsPage, profiles: profilesPage, advanced: advancedPage, about: aboutPage
            })[sw.page] || generalPage
        }
    }

    component Page: ColumnLayout {
        default property alias content: inner.data
        width: parent ? parent.width : 0
        ColumnLayout {
            id: inner
            Layout.fillWidth: true
            Layout.maximumWidth: 600
            Layout.alignment: Qt.AlignHCenter
            Layout.topMargin: 28
            Layout.bottomMargin: 36
            Layout.leftMargin: 28
            Layout.rightMargin: 28
            spacing: 24
            Txt {
                text: { for (var i = 0; i < sw.pages.length; i++) if (sw.pages[i].id === sw.page) return sw.pages[i].title; return "" }
                font.family: Theme.displayFont; font.pixelSize: Theme.headlineSize + 4
                color: Theme.surfaceFg
            }
        }
    }

    Component {
        id: generalPage
        Page {
            Section {
                title: qsTr("Startup")
                ChoiceRow { label: qsTr("OyNIx opens with"); key: "onLaunch"; options: [{ v: "restore", t: qsTr("All windows from last session") }, { v: "startpage", t: qsTr("A new window") }, { v: "homepage", t: qsTr("Homepage") }] }
                Divider {}
                ChoiceRow { label: qsTr("New tabs open with"); key: "newTabPage"; options: [{ v: "startpage", t: qsTr("Start Page") }, { v: "homepage", t: qsTr("Homepage") }, { v: "blank", t: qsTr("Empty Page") }] }
                Divider {}
                TextRow { label: qsTr("Homepage"); key: "homepage"; placeholder: "https://" }
            }
            Section {
                title: qsTr("Downloads")
                TextRow { label: qsTr("Save downloaded files to"); key: "downloadDir" }
                Divider {}
                SwitchRow { label: qsTr("Ask where to save each file"); key: "askWhereToSave" }
            }
            Section {
                title: qsTr("Default Browser")
                Row_ {
                    label: qsTr("Make OyNIx your default web browser")
                    PushButton { text: qsTr("Set as Default"); onClicked: text = Core.makeDefaultBrowser() ? qsTr("Done") : qsTr("Failed") }
                }
            }
            Section {
                title: qsTr("Import & Export")
                Row_ {
                    label: qsTr("Bookmarks")
                    hint: qsTr("HTML bookmark files from Firefox, Chrome, Safari and others")
                    PushButton { text: qsTr("Import…"); onClicked: importDialog.open() }
                    PushButton { text: qsTr("Export…"); onClicked: exportDialog.open() }
                }
            }
        }
    }

    FileDialog {
        id: importDialog
        title: qsTr("Import Bookmarks")
        nameFilters: [qsTr("Bookmark files (*.html *.htm)")]
        onAccepted: {
            var n = Core.importBookmarks(selectedFile.toString())
            importResult.text = n < 0 ? qsTr("Could not read that file.") : (n === 1 ? qsTr("Imported 1 bookmark.") : qsTr("Imported %1 bookmarks.").arg(n))
            importResult.open()
        }
    }
    FileDialog {
        id: exportDialog
        title: qsTr("Export Bookmarks")
        fileMode: FileDialog.SaveFile
        defaultSuffix: "html"
        nameFilters: [qsTr("Bookmark files (*.html)")]
        onAccepted: Core.exportBookmarks(selectedFile.toString())
    }
    MessageDialog { id: importResult; buttons: MessageDialog.Ok }

    // ── Extensions ───────────────────────────────────────────────────
    property string extMessage: ""
    property bool extMessageError: false
    function installExt(u) {
        var r = sw.app.extensionHost.install(u)
        extMessageError = !r.ok
        extMessage = r.ok ? qsTr("Installed “%1”").arg(r.extension.name) + (r.extension.warnings.length ? " — " + r.extension.warnings.join("; ") : "")
                          : qsTr("Could not install: %1").arg(r.error)
    }
    FileDialog {
        id: extFileDialog
        title: qsTr("Install Extension")
        nameFilters: [qsTr("Extensions (*.crx *.xpi *.zip *.json)"), qsTr("All files (*)")]
        onAccepted: sw.installExt(selectedFile.toString())
    }
    FolderDialog {
        id: extFolderDialog
        title: qsTr("Load Unpacked Extension")
        onAccepted: sw.installExt(selectedFolder.toString())
    }

    Component {
        id: appearancePage
        Page {
            Section {
                title: qsTr("Theme")
                Row_ {
                    label: qsTr("Appearance")
                    Repeater {
                        model: [{ v: "auto", t: qsTr("Auto") }, { v: "light", t: qsTr("Light") }, { v: "dark", t: qsTr("Dark") }]
                        delegate: AbstractButton {
                            required property var modelData
                            implicitWidth: 72; implicitHeight: 62
                            onClicked: sw.set("theme", modelData.v)
                            contentItem: Column {
                                spacing: 5
                                Rectangle {
                                    anchors.horizontalCenter: parent.horizontalCenter
                                    width: 56; height: 38; radius: 7
                                    border.width: Core.theme === modelData.v ? 2.5 : 1
                                    border.color: Core.theme === modelData.v ? Theme.accent : Theme.separator
                                    gradient: Gradient {
                                        orientation: Gradient.Horizontal
                                        GradientStop { position: 0; color: modelData.v === "dark" ? "#2a2a2d" : "#f3f3f4" }
                                        GradientStop { position: 0.5; color: modelData.v === "dark" ? "#2a2a2d" : "#f3f3f4" }
                                        GradientStop { position: 0.51; color: modelData.v === "light" ? "#f3f3f4" : "#2a2a2d" }
                                        GradientStop { position: 1; color: modelData.v === "light" ? "#f3f3f4" : "#2a2a2d" }
                                    }
                                }
                                Txt { anchors.horizontalCenter: parent.horizontalCenter; text: modelData.t; font.pixelSize: 12 }
                            }
                        }
                    }
                }
                Divider {}
                Row_ {
                    label: qsTr("Accent colour")
                    Repeater {
                        model: ["#2f7cf6", "#8e5cf7", "#e8457a", "#f2562e", "#f5a623", "#34c759", "#14b8a6", "#8e8e93"]
                        delegate: Rectangle {
                            required property string modelData
                            width: 20; height: 20; radius: 10
                            color: modelData
                            border.width: Core.accent === modelData ? 2 : 0
                            border.color: Theme.text
                            TapHandler { onTapped: sw.set("accent", modelData) }
                        }
                    }
                }
            }
            Section {
                title: qsTr("Websites")
                SwitchRow { label: qsTr("Use dark themes on websites"); hint: qsTr("When OyNIx is dark, sites that offer a dark theme show it (prefers-color-scheme)."); key: "darkWebsites" }
                Divider {}
                SwitchRow { label: qsTr("Darken all websites"); hint: qsTr("When OyNIx is dark, automatically darken sites that have no dark theme. Can be changed per site in Website settings."); key: "forceDarkWebsites" }
            }
            Section {
                title: qsTr("Toolbar")
                ChoiceRow { label: qsTr("Window controls"); key: "windowControlsSide"; options: [{ v: "left", t: qsTr("Left") }, { v: "right", t: qsTr("Right") }] }
                Divider {}
                ChoiceRow { label: qsTr("Open settings in"); key: "settingsOpenIn"; hint: qsTr("A window floats on tiling compositors such as Hyprland unless your window rules say otherwise (title “OyNIx Settings”).")
                            options: [{ v: "tab", t: qsTr("A tab") }, { v: "window", t: qsTr("A separate window") }] }
                Divider {}
                SwitchRow { label: qsTr("Use system title bar"); hint: qsTr("Let the window manager draw the window frame."); key: "useSystemTitleBar" }
                Divider {}
                SwitchRow { label: qsTr("Show Favorites bar"); key: "showFavoritesBar" }
                Divider {}
                SwitchRow { label: qsTr("Show full website address"); key: "showFullUrl" }
            }
            Section {
                title: qsTr("Start Page")
                SwitchRow { label: qsTr("Frequently Visited"); key: "startPageShowRecent" }
                Divider {}
                SwitchRow { label: qsTr("Privacy Report"); key: "startPageShowPrivacyReport" }
                Divider {}
                SwitchRow { label: qsTr("Reading List"); key: "startPageShowReadingList" }
                Divider {}
                ChoiceRow { label: qsTr("Background"); key: "startPageBackground"; options: [{ v: "default", t: qsTr("Soft gradient") }, { v: "plain", t: qsTr("Plain") }] }
            }
            Section {
                title: qsTr("Page Zoom")
                ChoiceRow { label: qsTr("Default zoom"); key: "defaultZoom"; options: [0.75, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5].map(function (z) { return { v: z, t: Math.round(z * 100) + "%" } }) }
            }
        }
    }

    Component {
        id: tabsPage
        Page {
            Section {
                title: qsTr("Tab Layout")
                Row_ {
                    label: qsTr("Tabs")
                    Repeater {
                        model: [{ v: "standard", t: qsTr("Standard"), i: "tabs" }, { v: "compact", t: qsTr("Compact"), i: "minus" }, { v: "vertical", t: qsTr("Vertical"), i: "sidebar" }]
                        delegate: AbstractButton {
                            required property var modelData
                            implicitWidth: 76; implicitHeight: 62
                            onClicked: sw.set("tabLayout", modelData.v)
                            contentItem: Column {
                                spacing: 5
                                Rectangle {
                                    anchors.horizontalCenter: parent.horizontalCenter
                                    width: 56; height: 38; radius: 7
                                    color: Theme.field
                                    border.width: Core.tabLayout === modelData.v ? 2.5 : 1
                                    border.color: Core.tabLayout === modelData.v ? Theme.accent : Theme.separator
                                    Icon { anchors.centerIn: parent; name: modelData.i; size: 20; color: Theme.textDim }
                                }
                                Txt { anchors.horizontalCenter: parent.horizontalCenter; text: modelData.t; font.pixelSize: 12 }
                            }
                        }
                    }
                }
            }
            Section {
                title: qsTr("Behaviour")
                SwitchRow { label: qsTr("Open links in background"); hint: qsTr("Middle-click and Ctrl-click open tabs without switching to them."); key: "openLinksInBackground" }
                Divider {}
                SwitchRow { label: qsTr("Close window when closing last tab"); key: "closeWindowWithLastTab" }
                Divider {}
                SwitchRow { label: qsTr("Show tab previews on hover"); key: "tabHoverPreviews" }
            }
            Section {
                title: qsTr("Memory")
                SwitchRow { label: qsTr("Unload inactive tabs"); hint: qsTr("Free memory used by tabs you haven’t looked at. They reload when selected."); key: "suspendInactiveTabs" }
                Divider {}
                ChoiceRow { label: qsTr("Unload after"); key: "suspendAfterMinutes"; options: [5, 15, 30, 60, 120, 240].map(function (m) { return { v: m, t: m < 60 ? qsTr("%1 minutes").arg(m) : (m === 60 ? qsTr("1 hour") : qsTr("%1 hours").arg(m / 60)) } }) }
                Divider {}
                SwitchRow { label: qsTr("Low Power Mode"); hint: qsTr("Freeze background tabs and turn off animations to save battery."); key: "lowPowerMode" }
            }
        }
    }

    Component {
        id: searchPage
        Page {
            Section {
                title: qsTr("Search Engine")
                ChoiceRow { label: qsTr("Default search engine"); key: "searchEngine"; options: JSON.parse(Core.searchEngines()).map(function (e) { return { v: e.id, t: e.name } }) }
                Divider {}
                SwitchRow { label: qsTr("Search suggestions"); hint: qsTr("Sends what you type to the search engine. Never in private windows."); key: "searchSuggestions" }
                Divider {}
                SwitchRow { label: qsTr("!Bang shortcuts"); hint: qsTr("Type “!w rust” to search Wikipedia, “!gh” for GitHub and so on."); key: "bangsEnabled" }
            }
            Section {
                title: qsTr("Available !bangs")
                Flow {
                    Layout.fillWidth: true
                    Layout.margins: 10
                    spacing: 6
                    Repeater {
                        model: JSON.parse(Core.bangs())
                        delegate: Rectangle {
                            required property var modelData
                            width: bt.implicitWidth + 14; height: 24; radius: 12; color: Theme.field
                            Txt { id: bt; anchors.centerIn: parent; text: "!" + modelData.key + "  " + modelData.name; font.pixelSize: 12 }
                        }
                    }
                }
            }
        }
    }

    Component {
        id: privacyPage
        Page {
            property var report: { Core.totalBlocked; return JSON.parse(Core.privacyReport()) }
            Section {
                title: qsTr("Privacy Report")
                RowLayout {
                    Layout.fillWidth: true
                    Layout.margins: 14
                    spacing: 24
                    Repeater {
                        model: [{ n: report.blocked, t: qsTr("Blocked") }, { n: report.trackers, t: qsTr("Trackers") }, { n: report.ads, t: qsTr("Ads") }]
                        delegate: ColumnLayout {
                            required property var modelData
                            spacing: 0
                            Txt { text: Number(modelData.n).toLocaleString(Qt.locale(), "f", 0); font.pixelSize: 26; font.weight: Font.Bold; color: Theme.accent }
                            Txt { text: modelData.t; color: Theme.textDim; font.pixelSize: 12 }
                        }
                    }
                    Item { Layout.fillWidth: true }
                    PushButton { text: qsTr("Reset"); onClicked: Core.resetPrivacyStats() }
                }
                Divider { visible: report.top.length > 0 }
                Repeater {
                    model: report.top
                    delegate: Row_ {
                        required property var modelData
                        label: modelData.domain
                        Txt { text: (modelData.sites === 1 ? qsTr("on 1 site") : qsTr("on %1 sites").arg(modelData.sites)); color: Theme.textDim; font.pixelSize: 12 }
                    }
                }
            }
            Section {
                title: qsTr("Content Blocker")
                SwitchRow {
                    label: qsTr("Block trackers and ads")
                    hint: report.loading ? qsTr("Loading filter lists…")
                          : qsTr("%1 network rules · %2 element-hiding rules · %3 lists").arg(Number(report.rules).toLocaleString(Qt.locale(), "f", 0)).arg(Number(report.cosmetic).toLocaleString(Qt.locale(), "f", 0)).arg(report.lists)
                    key: "contentBlocker"
                }
                Divider {}
                SwitchRow { label: qsTr("Block trackers"); key: "blockTrackers" }
                Divider {}
                SwitchRow { label: qsTr("Block ads"); key: "blockAds" }
                Divider {}
                SwitchRow { label: qsTr("Hide blocked page elements"); hint: qsTr("Applies the lists’ element-hiding rules, removing empty ad slots, banners and widgets."); key: "cosmeticFiltering" }
            }
            Section {
                id: listsSection
                title: qsTr("Filter Lists")
                property var lists: { Core.settingsJson; listsRefresh.tick; return JSON.parse(Core.filterLists()) }
                Timer { id: listsRefresh; property int tick: 0; interval: 1000; repeat: true; running: sw.shown && (sw.app.filterUpdating || report.loading); onTriggered: tick++ }
                Repeater {
                    model: [
                        { id: "ads", t: qsTr("Ads") }, { id: "privacy", t: qsTr("Privacy") },
                        { id: "annoyances", t: qsTr("Annoyances") }, { id: "cookies", t: qsTr("Cookie Notices") },
                        { id: "social", t: qsTr("Social Widgets") }, { id: "security", t: qsTr("Security") },
                        { id: "custom", t: qsTr("Custom") }
                    ]
                    delegate: ColumnLayout {
                        id: group
                        required property var modelData
                        readonly property var items: listsSection.lists.filter(function (l) { return l.category === group.modelData.id })
                        visible: items.length > 0
                        Layout.fillWidth: true
                        spacing: 0
                        Txt { text: group.modelData.t; color: Theme.textDim; font.pixelSize: 11; font.weight: Font.DemiBold; Layout.leftMargin: 12; Layout.topMargin: 8 }
                        Repeater {
                            model: group.items
                            delegate: Row_ {
                                required property var modelData
                                label: modelData.name
                                hint: (modelData.error ? "⚠ " + modelData.error + " · " : "")
                                      + (modelData.updated ? qsTr("%1 rules · updated %2").arg(Number(modelData.rules).toLocaleString(Qt.locale(), "f", 0)).arg(new Date(modelData.updated * 1000).toLocaleDateString(Qt.locale(), Locale.ShortFormat))
                                                           : (modelData.enabled ? qsTr("Not downloaded yet") : modelData.description))
                                IconButton {
                                    visible: modelData.category === "custom"
                                    glyph: "trash"; tip: qsTr("Remove")
                                    onClicked: Core.removeFilterList(modelData.id)
                                }
                                Toggle {
                                    checked: modelData.enabled
                                    onToggled: {
                                        Core.setFilterListEnabled(modelData.id, checked)
                                        if (checked && !modelData.downloaded) sw.app.updateFilterLists(false)
                                    }
                                }
                            }
                        }
                    }
                }
                Divider {}
                Row_ {
                    label: qsTr("Add a list by URL")
                    hint: qsTr("Any Adblock Plus, uBlock Origin or hosts-format list.")
                    Rectangle {
                        Layout.preferredWidth: 220; implicitHeight: 28; radius: 6; color: Theme.field; border.color: Theme.separator
                        TextInput {
                            id: listUrl
                            anchors.fill: parent; anchors.margins: 6
                            color: Theme.text; font.family: Theme.font; font.pixelSize: 12; clip: true
                            Txt { visible: !listUrl.text; text: "https://…"; color: Theme.textFaint; font.pixelSize: 12 }
                        }
                    }
                    PushButton {
                        text: qsTr("Add")
                        enabled: /^https?:\/\/./.test(listUrl.text.trim())
                        onClicked: { if (Core.addFilterListUrl(listUrl.text.trim())) { listUrl.text = ""; sw.app.updateFilterLists(false) } }
                    }
                }
                Divider {}
                Row_ {
                    label: sw.app.filterUpdating ? sw.app.filterStatus : qsTr("Update lists")
                    hint: qsTr("Lists refresh automatically every %1 days.").arg(sw.s.filterUpdateDays || 4)
                    PushButton { text: qsTr("Update Now"); enabled: !sw.app.filterUpdating; onClicked: sw.app.updateFilterLists(true) }
                }
                Divider {}
                Row_ {
                    label: qsTr("Local filter files")
                    hint: qsTr(".txt files in this folder are loaded as extra lists.")
                    PushButton { text: qsTr("Open Folder"); onClicked: Core.openExternally(Core.configPath("filters")) }
                    PushButton { text: qsTr("Reload"); onClicked: Core.reloadFilterLists() }
                }
            }
            Section {
                title: qsTr("Tracking Protection")
                SwitchRow { label: qsTr("Remove tracking parameters from links"); hint: qsTr("Strips utm_*, fbclid, gclid and similar."); key: "stripTrackingParams" }
                Divider {}
                SwitchRow { label: qsTr("Block third-party cookies"); key: "blockThirdPartyCookies" }
                Divider {}
                SwitchRow { label: qsTr("Fingerprinting protection"); key: "fingerprintProtection" }
                Divider {}
                SwitchRow { label: qsTr("Send Global Privacy Control"); key: "sendGpc" }
                Divider {}
                SwitchRow { label: qsTr("Send “Do Not Track”"); key: "sendDoNotTrack" }
                Divider {}
                SwitchRow { label: qsTr("Upgrade typed addresses to HTTPS"); key: "httpsUpgrade" }
            }
            Section {
                title: qsTr("Website Data")
                Row_ {
                    label: qsTr("Cache")
                    PushButton { text: qsTr("Clear Cache"); onClicked: { sw.app.defaultProfile.clearHttpCache(); text = qsTr("Cleared") } }
                }
                Divider {}
                Row_ {
                    label: qsTr("History")
                    PushButton { text: qsTr("Clear All History"); destructive: true; onClicked: { Core.clearHistory(0); text = qsTr("Cleared") } }
                }
            }
        }
    }

    Component {
        id: websitesPage
        Page {
            Section {
                title: qsTr("Defaults for all websites")
                SwitchRow { label: qsTr("JavaScript"); key: "javascript" }
                Divider {}
                SwitchRow { label: qsTr("Autoplay media with sound"); key: "autoplayMedia" }
                Divider {}
                SwitchRow { label: qsTr("Smooth scrolling"); key: "smoothScrolling" }
            }
            Section {
                title: qsTr("Customised websites")
                Repeater {
                    model: Object.keys(sw.s.sites || {})
                    delegate: Row_ {
                        required property string modelData
                        label: modelData
                        hint: JSON.stringify(sw.s.sites[modelData]).replace(/[{}"]/g, "").replace(/,/g, ", ").replace(/:/g, ": ")
                        IconButton { glyph: "trash"; onClicked: ["zoom", "blocker", "javascript", "popups", "reader"].forEach(function (k) { Core.setSiteSetting(modelData, k, "null") }) }
                    }
                }
                Txt { visible: Object.keys(sw.s.sites || {}).length === 0; text: qsTr("No websites have custom settings."); color: Theme.textDim; Layout.margins: 12 }
            }
        }
    }

    Component {
        id: extensionsPage
        Page {
            Section {
                title: qsTr("Built-in")
                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.margins: 12
                    spacing: 12
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 14
                        MShape { size: 48; shape: "cookie9"; color: Theme.primaryContainer
                            Icon { anchors.centerIn: parent; name: "sparkles"; size: 24; color: Theme.primaryContainerFg } }
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2
                            Txt { text: qsTr("Claude for OyNIx"); font.pixelSize: 16; font.weight: Font.DemiBold }
                            Txt { Layout.fillWidth: true; wrapMode: Text.WordWrap; elide: Text.ElideNone; color: Theme.textDim; font.pixelSize: 12
                                text: qsTr("Lets Claude read pages, click, type, navigate and manage tabs — from Claude Desktop or Claude Code with your Claude account (MCP), or from the Claude panel with an API key. Never active in private windows.") }
                        }
                        Toggle { checked: Core.agentEnabled; onToggled: Core.setSetting("agentEnabled", JSON.stringify(checked)) }
                    }
                    ColumnLayout {
                        visible: Core.agentEnabled
                        Layout.fillWidth: true
                        spacing: 10
                        Txt { text: qsTr("Site access"); font.weight: Font.DemiBold }
                        Segmented {
                            Layout.fillWidth: true
                            options: [{ v: "ask", t: qsTr("Ask before using a new site") }, { v: "auto", t: qsTr("Act on all sites except blocked") }]
                            value: Core.agentMode
                            onPicked: function (v) { Core.setSetting("agentMode", JSON.stringify(v)) }
                        }
                        Txt { text: qsTr("Allowed sites"); font.weight: Font.DemiBold; Layout.topMargin: 6 }
                        SiteList { key: "agentAllowedSites"; empty: qsTr("Sites you allow with “Always allow” appear here.") }
                        Txt { text: qsTr("Blocked sites"); font.weight: Font.DemiBold; Layout.topMargin: 6 }
                        SiteList { key: "agentBlockedSites"; empty: qsTr("Claude can never use blocked sites, for example your bank."); canAdd: true }
                        RowLayout {
                            Layout.topMargin: 6
                            spacing: 8
                            PushButton { text: qsTr("Open Claude Panel"); primary: true
                                onClicked: { var w = sw.app.windows[sw.app.windows.length - 1]; if (w) { w.sidebarOpen = true; w.raise(); w.requestActivate() } } }
                            Txt { color: Theme.textDim; font.pixelSize: 12
                                text: sw.app.agentBridge.listening ? qsTr("MCP endpoint ready · %1 connected").arg(sw.app.agentBridge.connections) : qsTr("MCP endpoint off") }
                        }
                    }
                }
            }
            Section {
                title: qsTr("Install")
                Row_ {
                    label: qsTr("Chrome & Firefox extensions")
                    hint: qsTr(".crx, .xpi or .zip files, or an unpacked folder. Manifest V2 and V3. Firefox extensions are adapted to Chromium automatically.")
                    PushButton { text: qsTr("From File…"); onClicked: extFileDialog.open() }
                    PushButton { text: qsTr("From Folder…"); onClicked: extFolderDialog.open() }
                }
                Divider {}
                Row_ {
                    label: qsTr("OyNIx extensions")
                    hint: qsTr("A folder with oynix.json, content.js and content.css — no build step.")
                    Rectangle {
                        Layout.preferredWidth: 150; implicitHeight: 28; radius: 6; color: Theme.field; border.color: Theme.separator
                        TextInput {
                            id: newExtName
                            anchors.fill: parent; anchors.margins: 6
                            color: Theme.text; font.family: Theme.font; font.pixelSize: 12; clip: true
                            Txt { visible: !newExtName.text; text: qsTr("Name"); color: Theme.textFaint; font.pixelSize: 12 }
                        }
                    }
                    PushButton {
                        text: qsTr("Create");
                        enabled: newExtName.text.trim() !== ""
                        onClicked: {
                            var dir = Core.createExtension(newExtName.text.trim())
                            if (dir) { Core.openExternally(dir); sw.extMessageError = false; sw.extMessage = qsTr("Created %1 — edit the files, then Reload").arg(dir); newExtName.text = "" }
                        }
                    }
                }
                Txt {
                    visible: sw.extMessage !== ""
                    Layout.fillWidth: true; Layout.margins: 12
                    wrapMode: Text.WordWrap; elide: Text.ElideNone
                    text: sw.extMessage
                    color: sw.extMessageError ? Theme.danger : Theme.success
                    font.pixelSize: 12
                }
                Txt {
                    visible: !sw.app.extensionHost.supported
                    Layout.fillWidth: true; Layout.margins: 12
                    wrapMode: Text.WordWrap; elide: Text.ElideNone
                    color: Theme.warning; font.pixelSize: 12
                    text: qsTr("This Qt build has no extension support (Qt WebEngine ≥ 6.10 is required). Only OyNIx extensions will run.")
                }
            }
            Section {
                title: qsTr("Installed")
                Repeater {
                    model: { Core.extensionsRevision; sw.app.extensionHost.revision; return JSON.parse(Core.extensions()) }
                    delegate: ColumnLayout {
                        id: ext
                        required property var modelData
                        required property int index
                        readonly property var host: sw.app.extensionHost
                        readonly property string err: host.errorFor(modelData.id)
                        Layout.fillWidth: true
                        spacing: 0
                        Divider { visible: ext.index > 0 }
                        RowLayout {
                            Layout.fillWidth: true
                            Layout.margins: 12
                            spacing: 12
                            Rectangle {
                                Layout.preferredWidth: 36; Layout.preferredHeight: 36; radius: 8; color: Theme.field
                                Image { anchors.centerIn: parent; width: 24; height: 24; sourceSize: Qt.size(48, 48); source: ext.modelData.icon ? "file://" + ext.modelData.icon : ""; visible: status === Image.Ready }
                                Icon { anchors.centerIn: parent; visible: !ext.modelData.icon; name: "puzzle"; size: 20; color: Theme.textDim }
                            }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2
                                RowLayout {
                                    spacing: 6
                                    Txt { text: ext.modelData.name; font.weight: Font.DemiBold; Layout.maximumWidth: 240 }
                                    Txt { text: ext.modelData.version; color: Theme.textDim; font.pixelSize: 11 }
                                    Rectangle {
                                        implicitWidth: kindText.implicitWidth + 12; implicitHeight: 18; radius: 9
                                        color: ext.modelData.kind === "firefox" ? "#33ff9f0a" : ext.modelData.kind === "oynix" ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.2) : "#330a84ff"
                                        Txt {
                                            id: kindText; anchors.centerIn: parent; font.pixelSize: 10; font.weight: Font.DemiBold
                                            text: ext.modelData.kind === "oynix" ? "OyNIx" : (ext.modelData.kind === "firefox" ? "Firefox" : "Chrome") + (ext.modelData.manifestVersion ? " · MV" + ext.modelData.manifestVersion : "")
                                        }
                                    }
                                }
                                Txt { visible: text !== ""; text: ext.modelData.description; color: Theme.textDim; font.pixelSize: 12; Layout.fillWidth: true; wrapMode: Text.WordWrap; elide: Text.ElideNone; maximumLineCount: 2 }
                                Txt {
                                    visible: ext.modelData.kind === "oynix"
                                    text: qsTr("Runs on: %1").arg(ext.modelData.matches.join(", "))
                                    color: Theme.textFaint; font.pixelSize: 11; Layout.fillWidth: true
                                }
                                Repeater {
                                    model: ext.modelData.warnings
                                    delegate: Txt { required property string modelData; text: "⚠ " + modelData; color: Theme.warning; font.pixelSize: 11; Layout.fillWidth: true; wrapMode: Text.WordWrap; elide: Text.ElideNone }
                                }
                                Txt { visible: ext.err !== ""; text: ext.err; color: Theme.danger; font.pixelSize: 11; Layout.fillWidth: true; wrapMode: Text.WordWrap; elide: Text.ElideNone }
                            }
                            IconButton {
                                glyph: "sliders"; tip: qsTr("Options")
                                visible: ext.modelData.optionsPage !== "" && ext.host.isLoaded(ext.modelData.id)
                                onClicked: { var w = sw.app.windows[sw.app.windows.length - 1]; if (w) { w.openTab(ext.host.optionsUrl(ext.modelData), {}); w.raise(); w.requestActivate() } }
                            }
                            IconButton {
                                glyph: "reload"; tip: qsTr("Reload from disk")
                                visible: ext.modelData.kind === "oynix"
                                onClicked: sw.installExt(ext.modelData.path)
                            }
                            IconButton { glyph: "folder"; tip: qsTr("Show Files"); onClicked: Core.openExternally(ext.modelData.path) }
                            IconButton { glyph: "trash"; tip: qsTr("Remove"); onClicked: ext.host.remove(ext.modelData.id) }
                            Toggle { checked: ext.modelData.enabled; onToggled: ext.host.setEnabled(ext.modelData.id, checked) }
                        }
                    }
                }
                Txt {
                    visible: Core.extensionsRevision >= 0 && JSON.parse(Core.extensions()).length === 0
                    text: qsTr("No extensions installed.")
                    color: Theme.textDim; Layout.margins: 12
                }
            }
            Section {
                title: qsTr("OyNIx extension format")
                Txt {
                    Layout.fillWidth: true; Layout.margins: 12
                    wrapMode: Text.WordWrap; elide: Text.ElideNone
                    color: Theme.textDim; font.pixelSize: 12
                    textFormat: Text.StyledText
                    text: qsTr("<b>oynix.json</b> — name, version, description, matches (e.g. *://*.example.com/*), exclude, run_at (document_start | document_end | document_idle), world (isolated | main).<br><b>content.js</b> — JavaScript run on matching pages.<br><b>content.css</b> — styles injected into matching pages (may be empty).<br>All three files are required. New page loads pick up changes.")
                }
            }
        }
    }

    Component {
        id: profilesPage
        Page {
            Section {
                title: qsTr("Profiles")
                Txt {
                    Layout.fillWidth: true; Layout.margins: 12
                    wrapMode: Text.WordWrap; color: Theme.textDim; font.pixelSize: 12
                    text: qsTr("Each profile has its own history, bookmarks, cookies, settings and website data, and runs as a separate app window. Current profile: %1").arg(Core.profile)
                }
                Repeater {
                    model: JSON.parse(Core.profiles())
                    delegate: Row_ {
                        required property string modelData
                        label: modelData + (modelData === Core.profile ? "  " + qsTr("(this window)") : "")
                        PushButton { text: qsTr("Open"); enabled: modelData !== Core.profile; onClicked: Core.launchProfile(modelData, false) }
                    }
                }
                Divider {}
                Row_ {
                    label: qsTr("New profile")
                    Rectangle {
                        Layout.preferredWidth: 180; implicitHeight: 28; radius: 6; color: Theme.field; border.color: Theme.separator
                        TextInput { id: pname; anchors.fill: parent; anchors.margins: 6; color: Theme.text; font.pixelSize: 12; clip: true }
                    }
                    PushButton { text: qsTr("Create & Open"); primary: true; enabled: pname.text.trim() !== ""; onClicked: { Core.launchProfile(pname.text.trim(), false); pname.text = "" } }
                }
            }
        }
    }

    Component {
        id: advancedPage
        Page {
            Section {
                title: qsTr("Developer")
                SwitchRow { label: qsTr("Show developer tools"); hint: qsTr("Adds Inspect Element and the Developer Tools menu item (F12)."); key: "developerMode" }
                Divider {}
                TextRow { label: qsTr("Custom user agent"); key: "userAgent"; placeholder: qsTr("Default") }
            }
            Section {
                title: qsTr("User Scripts & Styles")
                Row_ {
                    label: qsTr("Scripts folder")
                    hint: qsTr("Files ending in .user.js (Greasemonkey metadata supported) or .user.css are injected into matching pages.")
                    PushButton { text: qsTr("Open Folder"); onClicked: Core.openExternally(Core.configPath("userscripts")) }
                }
            }
            Section {
                title: qsTr("Rendering")
                Row_ { label: qsTr("Graphics"); Txt { text: Core.graphics; color: Theme.textDim; font.pixelSize: 12 } }
                Divider {}
                Row_ { label: qsTr("Choose with"); Txt { text: "oynix --gpu vulkan|gles|opengl|software"; color: Theme.textDim; font.family: "monospace"; font.pixelSize: 12 } }
            }
            Section {
                Row_ {
                    label: qsTr("Reset all settings")
                    PushButton { text: qsTr("Reset…"); destructive: true; onClicked: Core.resetSettings() }
                }
            }
        }
    }

    Component {
        id: aboutPage
        Page {
            ColumnLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: 10
                Image { Layout.alignment: Qt.AlignHCenter; source: "qrc:/qt/qml/OyNIx/assets/icon.png"; sourceSize: Qt.size(96, 96); width: 96; height: 96 }
                Txt { Layout.alignment: Qt.AlignHCenter; text: "OyNIx"; font.pixelSize: 26; font.weight: Font.Bold }
                Txt { Layout.alignment: Qt.AlignHCenter; text: qsTr("Version %1").arg(Core.version); color: Theme.textDim }
                Txt {
                    Layout.alignment: Qt.AlignHCenter
                    Layout.maximumWidth: 460
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.WordWrap
                    color: Theme.textDim
                    text: qsTr("A fast, private browser written in Rust and QML. Zero telemetry, built-in tracker and ad blocking.")
                }
            }
            Section {
                Row_ { label: qsTr("Graphics"); Txt { text: Core.graphics; color: Theme.textDim; font.pixelSize: 12 } }
                Divider {}
                Row_ { label: qsTr("Qt"); Txt { text: Core.qtVersion; color: Theme.textDim } }
                Divider {}
                Row_ { label: qsTr("Profile"); Txt { text: Core.profile; color: Theme.textDim } }
                Divider {}
                Row_ { label: qsTr("Configuration"); Txt { text: Core.configPath("config"); color: Theme.textDim; font.pixelSize: 12 } }
            }
        }
    }
}
