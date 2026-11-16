pragma Singleton
import QtQuick

// Scores for the offline games, kept for as long as OyNIx is open.
QtObject {
    property int goWins: 0
    property int goLosses: 0
    property int chips: 1000
    property int handsWon: 0
    property int handsLost: 0
}
