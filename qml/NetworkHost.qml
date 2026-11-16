import QtQuick
import QtQuick.Window
import QtWebEngine
import OyNIx

// Runs each enabled power extension's network.ts in a hidden page and
// carries out what it asks (Core.netCall / OutlineService.netFetch).
// Every message must carry the token that page was given, so nothing else
// on the WebChannel can speak for it. Private windows' requests never
// reach it (the interceptor for private windows doesn't report them).
Item {
    id: host
    property var app
    property var tokens: ({})
    property var views: ({})
    property var observing: ({})
    property int listeners: 0
    function recount() { var n = 0; for (var k in observing) if (observing[k]) n++; listeners = n }

    readonly property var exts: { Core.extensionsRevision; return JSON.parse(Core.extensions()).filter(function (e) { return e.enabled && e.power && e.hasNetwork }) }

    function newToken() {
        var s = ""
        for (var i = 0; i < 4; i++) s += Math.floor(Math.random() * 0x100000000).toString(36)
        return s + Date.now().toString(36)
    }
    function answer(ext, req, value, error) {
        var v = views[ext]
        if (v) v.runJavaScript("window.__oynixNet && __oynixNet.result(" + JSON.stringify(req) + ", " + JSON.stringify(value === undefined ? null : value) + ", " + JSON.stringify(error || "") + ")")
    }

    Window {
        id: hidden
        visible: false
        width: 16; height: 16
        Repeater {
            model: host.exts
            delegate: WebEngineView {
                id: nv
                required property var modelData
                width: 16; height: 16
                webChannel: host.app.extChannel
                webChannelWorld: WebEngineScript.MainWorld
                settings.localContentCanAccessFileUrls: false
                settings.javascriptCanOpenWindows: false
                settings.pluginsEnabled: false
                Component.onCompleted: {
                    var t = host.newToken()
                    host.tokens[modelData.id] = t
                    host.views[modelData.id] = nv
                    var html = Core.networkHtml(modelData.id, t)
                    if (html) loadHtml(html, "https://" + modelData.id + ".network.oynix.invalid/")
                }
                Component.onDestruction: {
                    var id = modelData.id
                    delete host.tokens[id]; delete host.views[id]; delete host.observing[id]; host.recount()
                    Core.netCall(id, JSON.stringify({ op: "forget" }))
                }
                onJavaScriptConsoleMessage: function (level, message, line, source) {
                    if (level >= WebEngineView.WarningMessageLevel) console.warn("network.ts [" + modelData.name + "]:", message)
                }
                // It has no business navigating anywhere.
                onNavigationRequested: function (request) {
                    var u = request.url.toString()
                    if (request.isMainFrame && u.indexOf("https://" + modelData.id + ".network.oynix.invalid/") !== 0 && u.indexOf("data:") !== 0) request.reject()
                }
            }
        }
    }

    Connections {
        target: host.app.extBridge
        function onMessage(ext, target, payload) {
            if (target !== "oynix:net") return
            var m
            try { m = JSON.parse(payload) } catch (e) { return }
            if (!host.tokens[ext] || m.token !== host.tokens[ext]) return    // not from that extension's page
            if (m.op === "fetch") {
                host.app.outlines.netFetch(ext, m.req, JSON.stringify({ url: m.url, method: m.method, headers: m.headers, body: m.body }))
                return
            }
            var call = { op: m.op }
            if (m.op === "block") call.rules = m.rules
            else if (m.op === "header") { call.pattern = m.pattern; call.name = m.name; call.value = m.value }
            else if (m.op === "observe") { call.on = m.on !== false; host.observing[ext] = call.on; host.recount() }
            var r = JSON.parse(Core.netCall(ext, JSON.stringify(call)))
            host.answer(ext, m.req, r.ok ? true : null, r.error)
        }
    }
    Connections {
        target: host.app.outlines
        function onNetFetched(ext, req, result) {
            var r = JSON.parse(result)
            host.answer(ext, req, r.error ? null : r, r.error)
        }
    }
    // Hand observed requests to the network.ts pages that listen.
    Timer {
        interval: 300; repeat: true
        running: host.listeners > 0
        onTriggered: {
            var list = JSON.parse(Core.netDrain())
            if (!list.length) return
            var js = "window.__oynixNet && __oynixNet.requests(" + JSON.stringify(list) + ")"
            for (var id in host.observing) if (host.observing[id] && host.views[id]) host.views[id].runJavaScript(js)
        }
    }
}
