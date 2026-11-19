import QtQuick
import QtQuick.Layouts
import OyNIx

// Blackjack against the dealer. Six-deck shoe, the dealer stands on all
// 17s, blackjack pays 3 to 2, you may double on your first two cards.
// Chips are play money and reset when OyNIx restarts (or when you run out).
ColumnLayout {
    id: bj
    spacing: 14

    property var shoe: []
    property var player: []
    property var dealer: []
    property int bet: 25
    property int stake: 0            // what's riding on this hand
    property string phase: "bet"     // bet | play | done
    property string status: qsTr("Place a bet and deal.")
    readonly property bool canDouble: phase === "play" && player.length === 2 && GameStats.chips >= stake

    function newShoe() {
        var s = []
        for (var d = 0; d < 6; d++) for (var suit = 0; suit < 4; suit++) for (var r = 1; r <= 13; r++) s.push({ r: r, s: suit })
        for (var i = s.length - 1; i > 0; i--) { var j = Math.floor(Math.random() * (i + 1)); var t = s[i]; s[i] = s[j]; s[j] = t }
        shoe = s
    }
    function draw() {
        if (shoe.length < 78) newShoe()   // reshuffle at three quarters through
        var s = shoe.slice(), c = s.pop()
        shoe = s
        return c
    }
    // {total, soft}
    function value(hand) {
        var t = 0, aces = 0
        for (var i = 0; i < hand.length; i++) { var r = hand[i].r; t += r === 1 ? 11 : Math.min(r, 10); if (r === 1) aces++ }
        while (t > 21 && aces > 0) { t -= 10; aces-- }
        return { total: t, soft: aces > 0 }
    }
    function isBlackjack(hand) { return hand.length === 2 && value(hand).total === 21 }

    function deal() {
        if (GameStats.chips <= 0) GameStats.chips = 1000
        var b = Math.min(bet, GameStats.chips)
        GameStats.chips -= b
        stake = b
        player = [draw(), draw()]
        dealer = [draw(), draw()]
        phase = "play"
        status = qsTr("Hit or stand?")
        if (isBlackjack(player) || isBlackjack(dealer)) settle()
    }
    function hit() {
        player = player.concat([draw()])
        var v = value(player).total
        if (v > 21) settle()
        else if (v === 21) stand()
    }
    function doubleDown() {
        GameStats.chips -= stake
        stake *= 2
        player = player.concat([draw()])
        if (value(player).total > 21) settle(); else stand()
    }
    function stand() {
        var d = dealer.slice()
        while (value(d).total < 17) d.push(draw())
        dealer = d
        settle()
    }
    function settle() {
        phase = "done"
        var p = value(player).total, d = value(dealer).total, pay = 0
        if (isBlackjack(player) && !isBlackjack(dealer)) { pay = stake + Math.floor(stake * 3 / 2); status = qsTr("Blackjack! You win %1.").arg(pay - stake) }
        else if (isBlackjack(dealer) && !isBlackjack(player)) status = qsTr("Dealer has blackjack.")
        else if (p > 21) status = qsTr("Bust. You lose %1.").arg(stake)
        else if (d > 21) { pay = stake * 2; status = qsTr("Dealer busts with %1. You win %2.").arg(d).arg(stake) }
        else if (p > d) { pay = stake * 2; status = qsTr("%1 beats %2. You win %3.").arg(p).arg(d).arg(stake) }
        else if (p < d) status = qsTr("Dealer’s %1 beats your %2.").arg(d).arg(p)
        else { pay = stake; status = qsTr("Push at %1. Your bet comes back.").arg(p) }
        GameStats.chips += pay
        if (pay > stake) GameStats.handsWon++
        else if (pay < stake) GameStats.handsLost++
        if (GameStats.chips <= 0) status += " " + qsTr("Out of chips; the next deal gives you 1,000 more.")
    }

    component Card: Rectangle {
        id: card
        property var c
        property bool hidden: false
        readonly property bool red: c && (c.s === 1 || c.s === 2)
        readonly property string rank: !c ? "" : ["", "A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K"][c.r]
        readonly property string suit: !c ? "" : ["♠", "♥", "♦", "♣"][c.s]
        width: 66; height: 94; radius: Theme.r(9)
        color: hidden ? Theme.primary : "#fbfaf7"
        border.width: Theme.edgeW(1); border.color: hidden ? Theme.primary : "#d6d2c8"
        Rectangle {   // card back pattern
            visible: card.hidden
            anchors.fill: parent; anchors.margins: 6; radius: Theme.r(6)
            color: "transparent"; border.width: Theme.edgeW(2); border.color: Theme.primaryFg; opacity: 0.5
        }
        Text { visible: !card.hidden; x: 7; y: 4; text: card.rank; font.pixelSize: 15; font.bold: true; color: card.red ? "#c62828" : "#1d1d1f" }
        Text { visible: !card.hidden; anchors.centerIn: parent; text: card.suit; font.pixelSize: 30; color: card.red ? "#c62828" : "#1d1d1f" }
        Text { visible: !card.hidden; anchors.right: parent.right; anchors.bottom: parent.bottom; anchors.margins: 6; text: card.rank; rotation: 180; font.pixelSize: 15; font.bold: true; color: card.red ? "#c62828" : "#1d1d1f" }
    }

    // ── Table ──
    RowLayout {
        Layout.fillWidth: true
        Txt { text: qsTr("Chips: %1").arg(GameStats.chips.toLocaleString(Qt.locale(), "f", 0)); font.weight: Font.DemiBold; font.pixelSize: 15 }
        Item { Layout.fillWidth: true }
        Txt { text: qsTr("Won %1 · Lost %2").arg(GameStats.handsWon).arg(GameStats.handsLost); color: Theme.textDim; font.pixelSize: 12 }
    }

    Rectangle {
        Layout.fillWidth: true
        implicitHeight: table.implicitHeight + 40
        radius: Theme.r(18)
        color: Theme.dark ? "#1f3a2c" : "#2e6b4c"
        ColumnLayout {
            id: table
            anchors.fill: parent; anchors.margins: 20
            spacing: 10
            Text {
                color: "#e8f2ec"; font.pixelSize: 13
                text: bj.phase === "play" ? qsTr("Dealer shows %1").arg(bj.dealer.length ? bj.value([bj.dealer[0]]).total : 0)
                    : bj.dealer.length ? qsTr("Dealer: %1").arg(bj.value(bj.dealer).total) : qsTr("Dealer")
            }
            Row {
                spacing: 8; Layout.preferredHeight: 94
                Repeater { model: bj.dealer; delegate: Card { required property var modelData; required property int index; c: modelData; hidden: index === 1 && bj.phase === "play" } }
            }
            Item { Layout.preferredHeight: 6 }
            Text {
                color: "#e8f2ec"; font.pixelSize: 13
                text: bj.player.length ? qsTr("You: %1%2").arg(bj.value(bj.player).total).arg(bj.value(bj.player).soft && bj.value(bj.player).total < 21 ? qsTr(" (soft)") : "") : qsTr("You")
            }
            Row {
                spacing: 8; Layout.preferredHeight: 94
                Repeater { model: bj.player; delegate: Card { required property var modelData; c: modelData } }
            }
        }
    }

    Txt { Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter; text: bj.status; wrapMode: Text.WordWrap; elide: Text.ElideNone; font.pixelSize: 14 }

    RowLayout {
        Layout.alignment: Qt.AlignHCenter
        spacing: 10
        visible: bj.phase !== "play"
        Txt { text: qsTr("Bet"); color: Theme.textDim }
        Repeater {
            model: [10, 25, 50, 100]
            delegate: PushButton { required property int modelData; text: modelData; primary: bj.bet === modelData; onClicked: bj.bet = modelData }
        }
        PushButton { text: qsTr("Deal"); primary: true; onClicked: bj.deal() }
    }
    RowLayout {
        Layout.alignment: Qt.AlignHCenter
        spacing: 10
        visible: bj.phase === "play"
        PushButton { text: qsTr("Hit"); primary: true; onClicked: bj.hit() }
        PushButton { text: qsTr("Stand"); onClicked: bj.stand() }
        PushButton { text: qsTr("Double"); enabled: bj.canDouble; onClicked: bj.doubleDown() }
    }

    Component.onCompleted: newShoe()
}
