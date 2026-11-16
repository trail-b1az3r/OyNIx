import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import OyNIx

// Go against the computer on a small board. Standard rules: stones with no
// liberties are captured, suicide isn't allowed, simple ko, and area
// scoring (stones + surrounded points) with komi for White. You play
// Black; two passes in a row end the game.
ColumnLayout {
    id: go
    spacing: 14

    property int size: 7
    property var board: []          // 0 empty, 1 black (you), 2 white (CPU)
    property int ko: -1             // point Black/White may not take back right now
    property int turn: 1
    property int lastMove: -1
    property int passes: 0
    property int capturedByYou: 0
    property int capturedByCpu: 0
    property bool over: false
    property string status: ""
    readonly property real komi: size >= 9 ? 5.5 : size >= 7 ? 4.5 : 2.5

    Component.onCompleted: newGame(size)

    function newGame(n) {
        size = n
        var b = []
        for (var i = 0; i < n * n; i++) b.push(0)
        board = b; ko = -1; turn = 1; lastMove = -1; passes = 0
        capturedByYou = 0; capturedByCpu = 0; over = false
        status = qsTr("Your move. You play Black.")
    }

    // ── Rules ──
    function neighbours(i, n) {
        var r = Math.floor(i / n), c = i % n, out = []
        if (r > 0) out.push(i - n)
        if (r < n - 1) out.push(i + n)
        if (c > 0) out.push(i - 1)
        if (c < n - 1) out.push(i + 1)
        return out
    }
    // {stones: [..], libs: count} for the group at i.
    function group(b, i, n) {
        var colour = b[i], seen = {}, libs = {}, stack = [i], stones = []
        seen[i] = true
        while (stack.length) {
            var p = stack.pop()
            stones.push(p)
            var ns = neighbours(p, n)
            for (var k = 0; k < ns.length; k++) {
                var q = ns[k]
                if (b[q] === 0) libs[q] = true
                else if (b[q] === colour && !seen[q]) { seen[q] = true; stack.push(q) }
            }
        }
        return { stones: stones, libs: Object.keys(libs).length }
    }
    // Play `colour` at i on board b. Returns null if illegal, else
    // {board, captured, ko}.
    function tryMove(b, i, colour, koPoint, n) {
        if (b[i] !== 0 || i === koPoint) return null
        var nb = b.slice()
        nb[i] = colour
        var other = 3 - colour, captured = [], ns = neighbours(i, n)
        for (var k = 0; k < ns.length; k++) {
            if (nb[ns[k]] !== other) continue
            var g = group(nb, ns[k], n)
            if (g.libs === 0) for (var s = 0; s < g.stones.length; s++) { if (nb[g.stones[s]] !== 0) { nb[g.stones[s]] = 0; captured.push(g.stones[s]) } }
        }
        var mine = group(nb, i, n)
        if (mine.libs === 0) return null   // suicide
        var newKo = captured.length === 1 && mine.stones.length === 1 && mine.libs === 1 ? captured[0] : -1
        return { board: nb, captured: captured.length, ko: newKo }
    }
    // Area score: stones plus empty regions touching only one colour.
    function score(b, n) {
        var black = 0, white = 0, seen = {}
        for (var i = 0; i < b.length; i++) {
            if (b[i] === 1) black++
            else if (b[i] === 2) white++
            else if (!seen[i]) {
                var stack = [i], region = 0, touches = {}
                seen[i] = true
                while (stack.length) {
                    var p = stack.pop(); region++
                    var ns = neighbours(p, n)
                    for (var k = 0; k < ns.length; k++) {
                        var q = ns[k]
                        if (b[q] === 0) { if (!seen[q]) { seen[q] = true; stack.push(q) } }
                        else touches[b[q]] = true
                    }
                }
                if (touches[1] && !touches[2]) black += region
                else if (touches[2] && !touches[1]) white += region
            }
        }
        return { black: black, white: white + komi }
    }

    // ── Computer player ──
    // Rough ownership: each empty point belongs to the nearest colour.
    function estimate(b, n) {
        var own = 0
        for (var i = 0; i < b.length; i++) {
            if (b[i] !== 0) { own += b[i] === 2 ? 1 : -1; continue }
            var r = Math.floor(i / n), c = i % n, best = 99, who = 0
            for (var j = 0; j < b.length; j++) {
                if (b[j] === 0) continue
                var d = Math.abs(Math.floor(j / n) - r) + Math.abs(j % n - c)
                if (d < best) { best = d; who = b[j] } else if (d === best && who !== b[j]) who = 0
            }
            if (best <= 3) own += who === 2 ? 1 : who === 1 ? -1 : 0
        }
        return own
    }
    function isOwnEye(b, i, colour, n) {
        var ns = neighbours(i, n)
        for (var k = 0; k < ns.length; k++) if (b[ns[k]] !== colour) return false
        for (k = 0; k < ns.length; k++) if (group(b, ns[k], n).libs === 1) return false
        return true
    }
    function cpuMove() {
        var n = size, b = board, before = estimate(b, n), best = -1, bestScore = -1e9
        var edgeGoal = n >= 9 ? 2 : 1
        for (var i = 0; i < b.length; i++) {
            if (b[i] !== 0 || isOwnEye(b, i, 2, n)) continue
            var r = tryMove(b, i, 2, ko, n)
            if (!r) continue
            var s = r.captured * 12
            var ns = neighbours(i, n)
            for (var k = 0; k < ns.length; k++) {
                var q = ns[k]
                if (b[q] === 2) {
                    var g0 = group(b, q, n)
                    if (g0.libs === 1 && group(r.board, i, n).libs >= 2) s += 9 * g0.stones.length      // saves a group
                } else if (b[q] === 1 && r.board[q] === 1) {
                    var g1 = group(r.board, q, n)
                    if (g1.libs === 1) s += 3 * g1.stones.length                                          // ataris
                }
            }
            var mine = group(r.board, i, n)
            if (mine.libs === 1 && r.captured === 0) s -= 14 * mine.stones.length                   // self-atari
            s += Math.min(mine.libs, 4) * 0.4
            var row = Math.floor(i / n), col = i % n
            var edge = Math.min(row, col, n - 1 - row, n - 1 - col)
            s -= Math.abs(edge - edgeGoal) * 0.35
            s += (estimate(r.board, n) - before) * 0.8
            s += Math.random() * 0.6
            if (s > bestScore) { bestScore = s; best = i }
        }
        // Pass when nothing helps, or when you passed and White is ahead.
        var sc = score(board, n)
        if (best < 0 || bestScore < 0.8 || (passes > 0 && sc.white > sc.black && bestScore < 3)) { pass(2); return }
        place(best, 2)
    }

    function place(i, colour) {
        var r = tryMove(board, i, colour, ko, size)
        if (!r) return false
        board = r.board; ko = r.ko; lastMove = i; passes = 0
        if (colour === 1) capturedByYou += r.captured; else capturedByCpu += r.captured
        turn = 3 - colour
        status = colour === 1 ? qsTr("White is thinking…") : qsTr("Your move.")
        if (turn === 2) cpuTimer.restart()
        return true
    }
    function pass(colour) {
        passes++
        lastMove = -1
        if (passes >= 2) { finish(); return }
        turn = 3 - colour
        status = colour === 1 ? qsTr("You passed. White is thinking…") : qsTr("White passed. Pass too to end the game, or keep playing.")
        if (turn === 2) cpuTimer.restart()
    }
    function finish() {
        over = true
        var sc = score(board, size)
        var diff = Math.abs(sc.black - sc.white)
        status = sc.black > sc.white ? qsTr("You win by %1 (Black %2, White %3 with komi).").arg(diff).arg(sc.black).arg(sc.white)
                                     : qsTr("White wins by %1 (Black %2, White %3 with komi).").arg(diff).arg(sc.black).arg(sc.white)
        if (sc.black > sc.white) GameStats.goWins++; else GameStats.goLosses++
    }
    Timer { id: cpuTimer; interval: 350; onTriggered: if (!go.over && go.turn === 2) go.cpuMove() }

    // ── Board ──
    RowLayout {
        Layout.fillWidth: true
        spacing: 8
        Repeater {
            model: [5, 7, 9]
            delegate: PushButton {
                required property int modelData
                text: modelData + "×" + modelData
                primary: go.size === modelData
                onClicked: go.newGame(modelData)
            }
        }
        Item { Layout.fillWidth: true }
        Txt { text: qsTr("Won %1 · Lost %2").arg(GameStats.goWins).arg(GameStats.goLosses); color: Theme.textDim; font.pixelSize: 12 }
    }

    Rectangle {
        id: boardRect
        readonly property real cell: Math.floor(Math.min(go.width, 420) / go.size)
        Layout.alignment: Qt.AlignHCenter
        Layout.preferredWidth: cell * go.size
        Layout.preferredHeight: cell * go.size
        radius: 14
        color: Theme.dark ? "#3a3226" : "#e3c58d"
        // Grid lines through the centres of the points.
        Repeater {
            model: go.size
            delegate: Item {
                required property int index
                Rectangle { x: boardRect.cell / 2; y: boardRect.cell / 2 + index * boardRect.cell; width: boardRect.cell * (go.size - 1); height: 1; color: Theme.dark ? "#8a7a5c" : "#7a6235" }
                Rectangle { y: boardRect.cell / 2; x: boardRect.cell / 2 + index * boardRect.cell; height: boardRect.cell * (go.size - 1); width: 1; color: Theme.dark ? "#8a7a5c" : "#7a6235" }
            }
        }
        Grid {
            columns: go.size
            Repeater {
                model: go.board.length
                delegate: Item {
                    id: pt
                    required property int index
                    width: boardRect.cell; height: boardRect.cell
                    readonly property int stone: go.board[index] || 0
                    Rectangle {
                        anchors.centerIn: parent
                        width: boardRect.cell * 0.86; height: width; radius: width / 2
                        visible: pt.stone !== 0 || (hov.hovered && go.turn === 1 && !go.over)
                        opacity: pt.stone === 0 ? 0.35 : 1
                        color: pt.stone === 2 ? "#f4f1ea" : "#1d1d1f"
                        border.width: pt.stone === 2 ? 1 : 0
                        border.color: "#9a948a"
                        Rectangle {   // marks the last move
                            anchors.centerIn: parent
                            visible: go.lastMove === pt.index
                            width: parent.width * 0.3; height: width; radius: width / 2
                            color: pt.stone === 1 ? "#f4f1ea" : "#1d1d1f"
                        }
                    }
                    HoverHandler { id: hov; cursorShape: pt.stone === 0 && go.turn === 1 ? Qt.PointingHandCursor : Qt.ArrowCursor }
                    TapHandler { onTapped: if (!go.over && go.turn === 1 && !go.place(pt.index, 1)) go.status = qsTr("You can’t play there (no liberties, or it would retake a ko).") }
                }
            }
        }
    }

    Txt { Layout.fillWidth: true; Layout.alignment: Qt.AlignHCenter; horizontalAlignment: Text.AlignHCenter; text: go.status; wrapMode: Text.WordWrap; elide: Text.ElideNone; font.pixelSize: 14 }
    Txt {
        Layout.alignment: Qt.AlignHCenter
        text: qsTr("Captured: you %1 · White %2 · komi %3").arg(go.capturedByYou).arg(go.capturedByCpu).arg(go.komi)
        color: Theme.textDim; font.pixelSize: 12
    }
    RowLayout {
        Layout.alignment: Qt.AlignHCenter
        spacing: 10
        PushButton { text: qsTr("Pass"); enabled: !go.over && go.turn === 1; onClicked: go.pass(1) }
        PushButton { text: qsTr("Resign"); enabled: !go.over; onClicked: { go.over = true; go.status = qsTr("You resigned. White wins."); GameStats.goLosses++ } }
        PushButton { text: qsTr("New Game"); primary: go.over; onClicked: go.newGame(go.size) }
    }
}
