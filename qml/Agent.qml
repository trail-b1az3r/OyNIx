import QtQuick
import QtWebEngine
import OyNIx

// "Claude for OyNIx": executes browser tools for Claude (from Claude Desktop
// / Claude Code over MCP, or from the built-in panel), enforcing the user's
// site permissions, and runs the built-in panel's conversation with the
// Claude API.
QtObject {
    id: agent
    property var app

    // ── State visible to the UI ─────────────────────────────────────
    property bool stopped: false
    property real lastActivity: 0
    property int activeTabUid: -1
    property string activeSource: ""          // "mcp" | "panel"
    property string currentAction: ""
    property var activity: []                 // [{time, source, tool, summary, host, ok}]
    property var prompt: null                 // pending permission request
    property var taskAllowed: ({})            // host -> true, for the current task
    readonly property bool working: lastActivity > 0 && (Date.now() - lastActivity) < 8000 && !stopped
    property Timer tick: Timer { interval: 1000; repeat: true; running: agent.lastActivity > 0; onTriggered: agent.lastActivityChanged() }
    property Timer taskExpiry: Timer { interval: 5 * 60 * 1000; onTriggered: agent.taskAllowed = {} }

    readonly property var settings: { Core.settingsJson; return JSON.parse(Core.settingsJson) }

    property Component timerComponent: Component { Timer {} }
    function later(ms, fn) {
        var t = timerComponent.createObject(agent, { interval: Math.max(1, ms) })
        t.triggered.connect(function () { t.destroy(); fn() })
        t.start()
    }

    function stop() {
        stopped = true
        if (prompt) { var p = prompt; prompt = null; p.answer("deny") }
        chatAbort()
    }
    function resume() { stopped = false }

    function log(source, tool, summary, host, ok) {
        var a = activity.slice(-199)
        a.push({ time: Date.now(), source: source, tool: tool, summary: summary, host: host || "", ok: ok })
        activity = a
    }

    // ── Tab resolution ──────────────────────────────────────────────
    function tabs() {
        var out = []
        for (var w = 0; w < app.windows.length; w++) {
            var win = app.windows[w]
            if (win.privateMode) continue
            for (var i = 0; i < win.tabs.count; i++) {
                var t = win.tabs.get(i)
                out.push({ win: win, index: i, uid: t.uid, title: t.title || "", url: t.url || "", active: i === win.currentIndex && win.active })
            }
        }
        return out
    }

    function focusedWindow() { return app.extensionHost.focusedWindow() }

    function resolve(tabId) {
        var list = tabs()
        if (tabId !== undefined && tabId !== null) {
            for (var i = 0; i < list.length; i++) if (list[i].uid === tabId) return list[i]
            return null
        }
        var fw = focusedWindow()
        if (!fw) return null
        for (var j = 0; j < list.length; j++) if (list[j].win === fw && list[j].index === fw.currentIndex) return list[j]
        return null
    }

    function siteOf(url) { return Core.hostOf(url || "") }

    function matchesList(host, list) {
        for (var i = 0; i < (list || []).length; i++) {
            var d = String(list[i]).toLowerCase().replace(/^\*\./, "")
            if (host === d || host.endsWith("." + d)) return true
        }
        return false
    }

    // ── Permissions ─────────────────────────────────────────────────
    function checkPermission(ctx, name, args, cont) {
        var host = ctx.host
        if (!host) { cont(true); return }                   // start page, about:blank
        if (matchesList(host, settings.agentBlockedSites)) { cont(false, host + " is on your blocked list for Claude."); return }
        if (settings.agentMode === "auto" || matchesList(host, settings.agentAllowedSites) || taskAllowed[host]) { cont(true); return }
        // Ask in the tab's window.
        ctx.win.raise(); ctx.win.requestActivate()
        prompt = {
            host: host, tool: name, source: ctx.source, win: ctx.win,
            detail: describeAction(name, args),
            answer: function (choice) {
                agent.prompt = null
                if (choice === "always") {
                    var l = (agent.settings.agentAllowedSites || []).slice(); l.push(host)
                    Core.setSetting("agentAllowedSites", JSON.stringify(l))
                    cont(true)
                } else if (choice === "task") {
                    var m = Object.assign({}, agent.taskAllowed); m[host] = true; agent.taskAllowed = m
                    agent.taskExpiry.restart()
                    cont(true)
                } else cont(false, "The user did not allow Claude to use " + host + ".")
            }
        }
        // Unanswered prompts deny after two minutes.
        var mine = prompt
        later(120000, function () { if (agent.prompt === mine) mine.answer("deny") })
    }

    function describeAction(name, a) {
        switch (name) {
        case "navigate": return qsTr("open %1").arg(a.url)
        case "click": return a.ref ? qsTr("click an element (%1)").arg(a.ref) : qsTr("click on the page")
        case "type_text": return qsTr("type “%1”").arg(String(a.text).slice(0, 40))
        case "press_key": return qsTr("press %1").arg(a.keys)
        case "form_input": return qsTr("fill in a form field")
        case "javascript": return qsTr("run JavaScript on the page")
        case "screenshot": return qsTr("take a screenshot")
        case "read_page": case "get_text": case "find": return qsTr("read the page")
        default: return name.replace(/_/g, " ")
        }
    }

    // ── Entry point ─────────────────────────────────────────────────
    // done(result) with result = {text?, image?, error?}
    function call(name, args, source, done) {
        args = args || {}
        if (!Core.agentEnabled) { done({ error: "Claude for OyNIx is turned off. The user can enable it in OyNIx: Settings › Extensions › Claude for OyNIx." }); return }
        if (stopped) { done({ error: "The user stopped Claude in OyNIx. Ask the user before continuing." }); return }
        var impl = tools[name]
        if (!impl) { done({ error: "Unknown tool: " + name }); return }
        lastActivity = Date.now()
        activeSource = source
        taskExpiry.restart()
        var global = ["tabs_list", "tab_open", "tab_switch", "tab_close", "history_search", "bookmarks"].indexOf(name) >= 0
        var ctx = { source: source }
        var finish = function (res) {
            agent.lastActivity = Date.now()
            agent.currentAction = ""
            var summary = res.error ? res.error : (res.summary || describeAction(name, args))
            agent.log(source, name, summary, ctx.host, !res.error)
            delete res.summary
            if (res.text && res.text.length > 60000) res.text = res.text.slice(0, 60000) + "\n… (truncated)"
            done(res)
        }
        if (global) { currentAction = describeAction(name, args); impl(ctx, args, finish); return }
        var t = resolve(args.tab_id)
        if (!t) { finish({ error: args.tab_id !== undefined ? "No tab with id " + args.tab_id + " (private tabs are not accessible)." : "No browser window is open." }); return }
        ctx.win = t.win; ctx.index = t.index; ctx.uid = t.uid; ctx.url = t.url; ctx.host = siteOf(t.url)
        // For navigate, the destination needs permission too.
        var dest = name === "navigate" && !/^(back|forward|reload)$/.test(args.url) ? siteOf(Core.resolveInput(args.url)) : ""
        checkPermission(ctx, name, args, function (ok, why) {
            if (!ok) { finish({ error: why }); return }
            var go = function () {
                if (agent.stopped) { finish({ error: "The user stopped Claude in OyNIx." }); return }
                agent.activeTabUid = ctx.uid
                agent.currentAction = agent.describeAction(name, args)
                impl(ctx, args, finish)
            }
            if (dest && dest !== ctx.host) {
                var dctx = { win: ctx.win, host: dest, source: source }
                agent.checkPermission(dctx, name, args, function (ok2, why2) { if (!ok2) finish({ error: why2 }); else go() })
            } else go()
        })
    }

    // ── Helpers for page tools ──────────────────────────────────────
    function tabItem(ctx) { return ctx.win.tabItem(ctx.index) }

    // Bring the tab to the front and wait until its web view exists.
    function front(ctx, cb, tries) {
        tries = tries || 0
        if (ctx.win.currentIndex !== ctx.index) ctx.win.activate(ctx.index)
        if (ctx.win.visibility === 3 /* minimized */) ctx.win.showNormal()
        var item = tabItem(ctx)
        if (item && item.view) { cb(item, item.view); return }
        if (item && item.internal) { cb(item, null); return }
        if (tries > 40) { cb(null, null); return }
        later(100, function () { agent.front(ctx, cb, tries + 1) })
    }

    function page(view, op, arg, cb) {
        var src = agentBridge().pageHelper() + "\n__oyA." + op + "(" + JSON.stringify(arg || {}) + ")"
        view.runJavaScript(src, WebEngineScript.ApplicationWorld, cb)
    }

    function agentBridge() { return app.agentBridge }

    function needView(ctx, done, cb) {
        front(ctx, function (item, view) {
            if (!view) { done({ error: "This tab shows a built-in OyNIx page with no web content. Navigate to a website first." }); return }
            cb(item, view)
        })
    }

    function waitLoaded(view, cb, ms, started) {
        ms = ms === undefined ? 30000 : ms
        if (!view) { cb(); return }
        if (view.loading && ms > 0) { later(200, function () { agent.waitLoaded(view, cb, ms - 200, true) }); return }
        cb()
    }

    function settle(view, cb) {
        // Give a click/keypress time to start a navigation, then wait for it.
        later(350, function () { agent.waitLoaded(view, function () { agent.later(150, cb) }, 15000) })
    }

    function cssToView(view, x, y) { var z = view.zoomFactor || 1; return { x: x * z, y: y * z } }

    function shotToView(item, x, y) { var s = item.agentShotScale || 1; return { x: x / s, y: y / s } }

    function modifierMask(s) {
        var m = 0, p = String(s || "").toLowerCase().split(/[+ ]/)
        for (var i = 0; i < p.length; i++) {
            if (p[i] === "ctrl" || p[i] === "control" || p[i] === "cmd" || p[i] === "meta" || p[i] === "command") m |= Qt.ControlModifier
            else if (p[i] === "shift") m |= Qt.ShiftModifier
            else if (p[i] === "alt" || p[i] === "option") m |= Qt.AltModifier
        }
        return m
    }

    readonly property var keyNames: ({
        "enter": Qt.Key_Return, "return": Qt.Key_Return, "tab": Qt.Key_Tab, "escape": Qt.Key_Escape, "esc": Qt.Key_Escape,
        "backspace": Qt.Key_Backspace, "delete": Qt.Key_Delete, "del": Qt.Key_Delete, "space": Qt.Key_Space, " ": Qt.Key_Space,
        "arrowup": Qt.Key_Up, "up": Qt.Key_Up, "arrowdown": Qt.Key_Down, "down": Qt.Key_Down, "arrowleft": Qt.Key_Left, "left": Qt.Key_Left,
        "arrowright": Qt.Key_Right, "right": Qt.Key_Right, "home": Qt.Key_Home, "end": Qt.Key_End, "pageup": Qt.Key_PageUp, "pagedown": Qt.Key_PageDown,
        "insert": Qt.Key_Insert, "f1": Qt.Key_F1, "f2": Qt.Key_F2, "f3": Qt.Key_F3, "f4": Qt.Key_F4, "f5": Qt.Key_F5, "f6": Qt.Key_F6,
        "f7": Qt.Key_F7, "f8": Qt.Key_F8, "f9": Qt.Key_F9, "f10": Qt.Key_F10, "f11": Qt.Key_F11, "f12": Qt.Key_F12
    })
    // Control characters are built at runtime (qmlcachegen mishandles them in literals).
    readonly property var keyText: ({ "enter": String.fromCharCode(13), "return": String.fromCharCode(13), "tab": String.fromCharCode(9),
                                      "space": " ", " ": " ", "backspace": String.fromCharCode(8), "escape": String.fromCharCode(27), "esc": String.fromCharCode(27) })

    // Parse "ctrl+a" / "Enter" / "shift+Tab" into {key, mods, text}.
    function parseChord(chord) {
        var parts = chord.split("+").filter(function (p) { return p !== "" })
        if (chord.endsWith("++")) parts.push("+")
        var keyName = parts.pop() || ""
        var mods = modifierMask(parts.join("+"))
        var k = keyName.toLowerCase()
        if (keyNames[k] !== undefined) return { key: keyNames[k], mods: mods, text: (mods & Qt.ControlModifier) ? "" : (keyText[k] || "") }
        if (keyName.length === 1) {
            var up = keyName.toUpperCase()
            var code = up.charCodeAt(0)
            var text = (mods & (Qt.ControlModifier | Qt.AltModifier)) ? "" : ((mods & Qt.ShiftModifier) ? up : keyName)
            return { key: code, mods: mods, text: text }
        }
        return null
    }

    function clickAt(ctx, item, view, x, y, args, label, done) {
        var button = args.button === "right" ? 2 : args.button === "middle" ? 4 : 1
        var urlBefore = view.url.toString()
        Core.sendMouse(view, x, y, button, args.double ? 2 : 1, modifierMask(args.modifiers), 1)
        settle(view, function () {
            var after = view.url.toString()
            done({ text: "Clicked " + label + "." + (after !== urlBefore ? " The page navigated to " + after + " (" + view.title + ")." : "") })
        })
    }

    function elementPoint(view, ref, cb) {
        page(view, "rect", { ref: ref }, function (r) {
            if (!r || r.error) { cb(null, r ? r.error : "Could not locate the element."); return }
            // Let instant scrolling settle before using coordinates.
            agent.later(60, function () { agent.page(view, "rect", { ref: ref }, function (r2) { cb(r2 && !r2.error ? r2 : r, null) }) })
        })
    }

    // ── Tool implementations ────────────────────────────────────────
    readonly property var tools: ({
        tabs_list: function (ctx, a, done) {
            var list = agent.tabs().map(function (t) { return { tab_id: t.uid, title: t.title, url: t.url, active: t.active } })
            done({ text: JSON.stringify(list, null, 1), summary: qsTr("listed %1 tabs").arg(list.length) })
        },
        tab_open: function (ctx, a, done) {
            var w = agent.focusedWindow() || app.newWindow({})
            var url = a.url ? Core.resolveInput(a.url) : w.newTabUrl()
            var host = agent.siteOf(url)
            if (host && agent.matchesList(host, agent.settings.agentBlockedSites)) { done({ error: host + " is on your blocked list for Claude." }); return }
            var finishOpen = function () {
                var idx = w.openTab(url, { background: !!a.background, afterCurrent: true })
                var uid = w.tabs.get(idx).uid
                agent.later(300, function () {
                    var it = w.tabItem(idx)
                    agent.waitLoaded(it ? it.view : null, function () { done({ text: JSON.stringify({ tab_id: uid, url: url }), summary: qsTr("opened %1").arg(host || qsTr("a new tab")) }) }, 30000)
                })
            }
            if (host) agent.checkPermission({ win: w, host: host, source: ctx.source }, "navigate", { url: url }, function (ok, why) { if (ok) finishOpen(); else done({ error: why }) })
            else finishOpen()
        },
        tab_switch: function (ctx, a, done) {
            var t = agent.resolve(a.tab_id); if (!t) { done({ error: "No tab with id " + a.tab_id }); return }
            t.win.activate(t.index); t.win.raise(); t.win.requestActivate()
            done({ text: "Switched to tab " + a.tab_id + ": " + t.title })
        },
        tab_close: function (ctx, a, done) {
            var t = agent.resolve(a.tab_id); if (!t) { done({ error: "No tab with id " + a.tab_id }); return }
            t.win.closeTab(t.index); done({ text: "Closed tab " + a.tab_id + "." })
        },
        navigate: function (ctx, a, done) {
            agent.front(ctx, function (item, view) {
                if (!item) { done({ error: "The tab is not available." }); return }
                var u = String(a.url || "")
                if (u === "back" || u === "forward" || u === "reload") {
                    if (!view) { done({ error: "Nothing to go " + u + " to." }); return }
                    if (u === "back") view.goBack(); else if (u === "forward") view.goForward(); else view.reload()
                } else item.load(Core.resolveInput(u))
                agent.later(400, function () {
                    var v = item.view
                    agent.waitLoaded(v, function () {
                        done({ text: "Loaded " + (v ? v.url.toString() + " — " + v.title : item.model.url) + (item.loadError ? " (error: " + item.loadError + ")" : ""),
                               summary: qsTr("opened %1").arg(agent.siteOf(v ? v.url.toString() : u) || u) })
                    })
                })
            })
        },
        page_info: function (ctx, a, done) {
            agent.needView(ctx, done, function (item, view) {
                agent.page(view, "info", {}, function (r) {
                    r = r || {}
                    r.tab_id = ctx.uid; r.loading = view.loading; r.zoom = view.zoomFactor
                    done({ text: JSON.stringify(r, null, 1) })
                })
            })
        },
        read_page: function (ctx, a, done) {
            agent.needView(ctx, done, function (item, view) {
                agent.waitLoaded(view, function () {
                    agent.page(view, "snapshot", { filter: a.filter || "all", max_chars: a.max_chars || 30000 }, function (r) {
                        done(r ? { text: r, summary: qsTr("read %1").arg(ctx.host || qsTr("the page")) } : { error: "Could not read this page." })
                    })
                }, 10000)
            })
        },
        find: function (ctx, a, done) {
            agent.needView(ctx, done, function (item, view) {
                agent.page(view, "find", { query: a.query }, function (r) { done({ text: r || "Nothing found.", summary: qsTr("searched the page for “%1”").arg(a.query) }) })
            })
        },
        get_text: function (ctx, a, done) {
            agent.needView(ctx, done, function (item, view) {
                agent.waitLoaded(view, function () {
                    agent.page(view, "text", { max_chars: a.max_chars || 40000 }, function (r) { done(r ? { text: r, summary: qsTr("read %1").arg(ctx.host) } : { error: "Could not read this page." }) })
                }, 10000)
            })
        },
        screenshot: function (ctx, a, done) {
            agent.front(ctx, function (item, view) {
                var target = view || (item ? item.pageItem : null)
                if (!target) { done({ error: "Nothing to capture." }); return }
                agent.later(250, function () {
                    var s = Math.min(1, 1280 / Math.max(1, target.width))
                    item.agentShotScale = s
                    var path = agent.agentBridge().tempImagePath()
                    target.grabToImage(function (res) {
                        if (!res.saveToFile(path)) { done({ error: "Could not capture the tab." }); return }
                        done({ image: path, text: "Screenshot of " + (view ? view.url.toString() : "the start page") + ", " + Math.round(target.width * s) + "x" + Math.round(target.height * s) + " px. Use these coordinates for click/hover x,y.",
                               summary: qsTr("took a screenshot of %1").arg(ctx.host || qsTr("the tab")) })
                    }, Qt.size(Math.round(target.width * s), Math.round(target.height * s)))
                })
            })
        },
        click: function (ctx, a, done) {
            agent.needView(ctx, done, function (item, view) {
                if (a.ref) {
                    agent.elementPoint(view, a.ref, function (r, err) {
                        if (!r) { done({ error: err }); return }
                        var p = agent.cssToView(view, r.x, r.y)
                        agent.clickAt(ctx, item, view, p.x, p.y, a, r.label + (r.covered ? " (note: it was covered by “" + r.cover + "”)" : ""), done)
                    })
                } else if (a.x !== undefined && a.y !== undefined) {
                    var p = agent.shotToView(item, a.x, a.y)
                    agent.clickAt(ctx, item, view, p.x, p.y, a, "at " + a.x + "," + a.y, done)
                } else done({ error: "Give either ref or x and y." })
            })
        },
        hover: function (ctx, a, done) {
            agent.needView(ctx, done, function (item, view) {
                var go = function (x, y, label) { Core.sendMouse(view, x, y, 0, 0, 0, 0); agent.later(300, function () { done({ text: "Hovering over " + label + "." }) }) }
                if (a.ref) agent.elementPoint(view, a.ref, function (r, err) { if (!r) done({ error: err }); else { var p = agent.cssToView(view, r.x, r.y); go(p.x, p.y, r.label) } })
                else if (a.x !== undefined) { var p = agent.shotToView(item, a.x, a.y); go(p.x, p.y, a.x + "," + a.y) }
                else done({ error: "Give either ref or x and y." })
            })
        },
        type_text: function (ctx, a, done) {
            agent.needView(ctx, done, function (item, view) {
                var typeNow = function () {
                    if (a.clear) { Core.sendKey(view, Qt.Key_A, Qt.ControlModifier, ""); Core.sendKey(view, Qt.Key_Backspace, 0, "") }
                    Core.sendText(view, String(a.text))
                    if (a.submit) {
                        Core.sendKey(view, Qt.Key_Return, 0, String.fromCharCode(13))
                        agent.settle(view, function () { done({ text: "Typed the text and pressed Enter. Page: " + view.url.toString(), summary: qsTr("typed and submitted") }) })
                    } else agent.later(150, function () { done({ text: "Typed " + String(a.text).length + " characters.", summary: qsTr("typed text") }) })
                }
                if (a.ref) {
                    agent.elementPoint(view, a.ref, function (r, err) {
                        if (!r) { done({ error: err }); return }
                        var p = agent.cssToView(view, r.x, r.y)
                        Core.sendMouse(view, p.x, p.y, 1, 1, 0, 1)
                        agent.later(120, function () { agent.page(view, "focus", { ref: a.ref }, function () { typeNow() }) })
                    })
                } else typeNow()
            })
        },
        press_key: function (ctx, a, done) {
            agent.needView(ctx, done, function (item, view) {
                var chords = String(a.keys || "").trim().split(/\s+/)
                for (var i = 0; i < chords.length; i++) {
                    var c = agent.parseChord(chords[i])
                    if (!c) { done({ error: "Unknown key: " + chords[i] }); return }
                    Core.sendKey(view, c.key, c.mods, c.text)
                }
                agent.settle(view, function () { done({ text: "Pressed " + a.keys + ".", summary: qsTr("pressed %1").arg(a.keys) }) })
            })
        },
        scroll: function (ctx, a, done) {
            agent.needView(ctx, done, function (item, view) {
                var report = function (msg) { agent.later(350, function () { agent.page(view, "info", {}, function (r) { done({ text: msg + (r ? " Now at " + r.scrollY + " of " + r.scrollHeight + " px." : "") }) }) }) }
                if (a.to_ref) { agent.page(view, "rect", { ref: a.to_ref }, function (r) { if (!r || r.error) done({ error: r ? r.error : "Not found" }); else report("Scrolled " + r.label + " into view.") }); return }
                var dir = a.direction || "down", amt = a.amount || 0.8
                var dx = 0, dy = 0
                if (dir === "down") dy = view.height * amt; else if (dir === "up") dy = -view.height * amt
                else if (dir === "right") dx = view.width * amt; else dx = -view.width * amt
                var at = function (x, y) {
                    var steps = Math.max(1, Math.round(Math.max(Math.abs(dx), Math.abs(dy)) / 100))
                    for (var i = 0; i < steps; i++) Core.sendWheel(view, x, y, dx / steps, dy / steps)
                    report("Scrolled " + dir + ".")
                }
                if (a.ref) agent.elementPoint(view, a.ref, function (r, err) { if (!r) done({ error: err }); else { var p = agent.cssToView(view, r.x, r.y); at(p.x, p.y) } })
                else at(view.width / 2, view.height / 2)
            })
        },
        form_input: function (ctx, a, done) {
            agent.needView(ctx, done, function (item, view) {
                agent.page(view, "setValue", { ref: a.ref, value: a.value }, function (r) { done({ text: r || "Done.", summary: qsTr("filled in a form field") }) })
            })
        },
        wait: function (ctx, a, done) {
            if (a.seconds && !a.for_text && !a.for_selector) { agent.later(Math.min(60, a.seconds) * 1000, function () { done({ text: "Waited " + a.seconds + " s." }) }); return }
            agent.needView(ctx, done, function (item, view) {
                var deadline = Date.now() + Math.min(120, a.timeout || 10) * 1000
                var poll = function () {
                    agent.page(view, "has", a, function (r) {
                        if (r === true) done({ text: "Found it." })
                        else if (r === "bad selector") done({ error: "Invalid CSS selector." })
                        else if (Date.now() > deadline) done({ error: "Timed out waiting." })
                        else agent.later(300, poll)
                    })
                }
                poll()
            })
        },
        javascript: function (ctx, a, done) {
            agent.needView(ctx, done, function (item, view) {
                view.runJavaScript(String(a.code), function (r) {
                    var out
                    try { out = r === undefined ? "undefined" : JSON.stringify(r, null, 1) } catch (e) { out = String(r) }
                    done({ text: out || "null", summary: qsTr("ran JavaScript on %1").arg(ctx.host) })
                })
            })
        },
        console_logs: function (ctx, a, done) {
            var item = agent.tabItem(ctx)
            var logs = (item && item.consoleLog) ? item.consoleLog : []
            if (a.errors_only) logs = logs.filter(function (l) { return l.level >= 2 })
            logs = logs.slice(-(a.limit || 100))
            var names = ["info", "warning", "error"]
            done({ text: logs.length ? logs.map(function (l) { return "[" + names[Math.min(2, l.level)] + "] " + l.message + (l.source ? "  (" + l.source + ":" + l.line + ")" : "") }).join("\n") : "No console messages." })
        },
        network_requests: function (ctx, a, done) {
            var list = JSON.parse(agent.agentBridge().networkRequests(ctx.host, a.url_contains || "", a.limit || 100))
            done({ text: list.length ? list.map(function (r) { return (r.blocked ? "BLOCKED " : "") + r.type + " " + r.url }).join("\n") : "No requests recorded for " + ctx.host + "." })
        },
        history_search: function (ctx, a, done) {
            var h = JSON.parse(Core.history(a.query || "", a.limit || 25))
            done({ text: JSON.stringify(h.map(function (x) { return { title: x.title, url: x.url, visits: x.visits, last_visit: new Date(x.lastVisit * 1000).toISOString() } }), null, 1) })
        },
        bookmarks: function (ctx, a, done) {
            var q = String(a.query || "").toLowerCase()
            var b = JSON.parse(Core.bookmarks("*")).filter(function (x) { return !q || x.title.toLowerCase().indexOf(q) >= 0 || x.url.toLowerCase().indexOf(q) >= 0 })
            done({ text: JSON.stringify(b.map(function (x) { return { title: x.title, url: x.url, folder: x.folder || "Bookmarks" } }), null, 1) })
        }
    })

    // ── Built-in panel: conversation with the Claude API ─────────────
    property var messages: []      // API conversation (append-only)
    property var chat: []          // display log
    property bool chatBusy: false
    property string chatError: ""
    property var xhr: null
    property int retries: 0
    readonly property var models: [
        { id: "claude-opus-5-5", name: "Claude Opus 5.5" },
        { id: "claude-sonnet-5-5", name: "Claude Sonnet 5.5" },
        { id: "claude-haiku-5-5", name: "Claude Haiku 5.5" }
    ]

    function chatPush(entry) { var c = chat.slice(); c.push(entry); chat = c; return c.length - 1 }
    function chatUpdate(i, patch) { var c = chat.slice(); c[i] = Object.assign({}, c[i], patch); chat = c }

    function newChat() {
        chatAbort()
        messages = []; chat = []; chatError = ""; taskAllowed = {}
    }

    function apiTools() {
        return JSON.parse(agentBridge().tools()).map(function (t) { return { name: t.name, description: t.description, input_schema: t.input_schema } })
    }

    // Close any tool_use left without results (e.g. after Stop) so the
    // conversation stays valid; history is only ever appended to.
    function closeDanglingTools() {
        if (!messages.length) return
        var last = messages[messages.length - 1]
        if (last.role !== "assistant") return
        var uses = (last.content || []).filter(function (b) { return b.type === "tool_use" })
        if (!uses.length) return
        var m = messages.slice()
        m.push({ role: "user", content: uses.map(function (u) { return { type: "tool_result", tool_use_id: u.id, content: "Stopped by the user before this ran.", is_error: true } }) })
        messages = m
    }

    function sendChat(text) {
        text = String(text || "").trim()
        if (!text || chatBusy) return
        if (!Core.agentEnabled) { chatError = qsTr("Turn on Claude for OyNIx first."); return }
        resume()
        chatError = ""
        closeDanglingTools()
        var t = resolve(undefined)
        var context = t ? "\n\n<browser_context>Active tab " + t.uid + ": " + t.title + " — " + t.url + "</browser_context>" : ""
        var m = messages.slice()
        m.push({ role: "user", content: [{ type: "text", text: text + context }] })
        messages = m
        chatPush({ kind: "user", text: text })
        retries = 0
        request()
    }

    function chatAbort() {
        if (xhr) { var x = xhr; xhr = null; x.abort() }
        if (chatBusy) chatPush({ kind: "note", text: qsTr("Stopped.") })
        chatBusy = false
    }

    function request() {
        var key = agentBridge().apiKey()
        if (!key) { chatError = qsTr("Add an API key in Settings › Extensions › Claude for OyNIx, or use Claude Desktop / Claude Code."); return }
        chatBusy = true
        var model = settings.agentModel || "claude-opus-5-5"
        var body = {
            model: model,
            max_tokens: 16000,
            system: agentBridge().systemPrompt(),
            tools: apiTools(),
            messages: messages,
            output_config: { effort: settings.agentEffort || "high" },
            cache_control: { type: "ephemeral" },
            context_management: { edits: [{ type: "clear_tool_uses_20250919" }] }
        }
        var betas = ["context-management-2025-06-27"]
        // Server-side fallback on refusals (Opus / Sonnet 5.5).
        if (model !== "claude-haiku-5-5") { body.fallbacks = "default"; betas.push("server-side-fallback-2026-07-01") }
        var x = new XMLHttpRequest()
        xhr = x
        x.onreadystatechange = function () {
            if (x.readyState !== XMLHttpRequest.DONE || agent.xhr !== x) return
            agent.xhr = null
            agent.handleResponse(x.status, x.responseText)
        }
        x.open("POST", "https://api.anthropic.com/v1/messages")
        x.setRequestHeader("content-type", "application/json")
        x.setRequestHeader("x-api-key", key)
        x.setRequestHeader("anthropic-version", "2026-10-21")
        x.setRequestHeader("anthropic-beta", betas.join(","))
        x.send(JSON.stringify(body))
    }

    function handleResponse(status, text) {
        var r = null
        try { r = JSON.parse(text) } catch (e) {}
        if (status !== 200 || !r || r.type === "error") {
            var msg = r && r.error ? r.error.message : (status ? "HTTP " + status : qsTr("Network error"))
            if ((status === 429 || status === 529 || status >= 500) && retries < 3) {
                retries++
                chatPush({ kind: "note", text: qsTr("Claude is busy, retrying…") })
                later(2000 * retries, request)
                return
            }
            chatBusy = false
            chatError = status === 401 ? qsTr("The API key was rejected. Check it in Settings › Extensions.") : msg
            return
        }
        retries = 0
        var m = messages.slice()
        m.push({ role: "assistant", content: r.content })
        messages = m
        for (var i = 0; i < r.content.length; i++) {
            var b = r.content[i]
            if (b.type === "text" && b.text.trim()) chatPush({ kind: "assistant", text: b.text })
        }
        if (r.stop_reason === "tool_use") { runTools(r.content.filter(function (b) { return b.type === "tool_use" })); return }
        chatBusy = false
        if (r.stop_reason === "refusal") chatPush({ kind: "note", text: qsTr("Claude declined this request.") + (r.stop_details && r.stop_details.explanation ? " " + r.stop_details.explanation : "") })
        else if (r.stop_reason === "max_tokens") chatPush({ kind: "note", text: qsTr("The reply hit the length limit.") })
        else if (r.stop_reason === "pause_turn") { chatBusy = true; request() }
    }

    function runTools(uses) {
        var results = []
        var i = 0
        var nextTool = function () {
            if (!agent.chatBusy) return               // stopped
            if (i >= uses.length) {
                var m = agent.messages.slice()
                m.push({ role: "user", content: results })
                agent.messages = m
                agent.request()
                return
            }
            var u = uses[i++]
            var row = agent.chatPush({ kind: "tool", tool: u.name, text: agent.describeAction(u.name, u.input || {}), status: "running" })
            agent.call(u.name, u.input || {}, "panel", function (res) {
                agent.chatUpdate(row, { status: res.error ? "error" : "done", detail: res.error || "" })
                var content = []
                if (res.error) content.push({ type: "text", text: res.error })
                if (res.text) content.push({ type: "text", text: res.text })
                if (res.image) {
                    var data = agent.agentBridge().takeImageBase64(res.image)
                    if (data) content.push({ type: "image", source: { type: "base64", media_type: "image/jpeg", data: data } })
                }
                if (!content.length) content.push({ type: "text", text: "OK" })
                results.push({ type: "tool_result", tool_use_id: u.id, content: content, is_error: !!res.error })
                nextTool()
            })
        }
        nextTool()
    }
}
