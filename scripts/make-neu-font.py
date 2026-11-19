#!/usr/bin/env python3
"""Build OyNIx Neu, the neumorphic typeface of OyNIx's Neumorphic and
Graphite styles.

    pip install fonttools shapely
    python3 scripts/make-neu-font.py

Every glyph is a skeleton of lines and arcs, stroked with round caps and
joins (shapely) and written as TrueType outlines (fontTools). Two files:

  assets/fonts/OyNIxNeu-Regular.ttf  the outlines; OyNIx draws its soft
                                     light, shadow and depth in QML
                                     (NeuTextEffect.qml)
  assets/fonts/OyNIxNeu-Color.ttf    the same outlines plus a COLRv1 colour
                                     table: each glyph carries its own
                                     2.5D look (a stacked soft shadow, an
                                     extruded side, a light rim and a
                                     gradient face), for web pages; it
                                     falls back to plain outlines where
                                     COLRv1 isn't supported

The design: a soft, round, geometric monoline (stroke 92/1000 em), with
open apertures and generous spacing so it stays readable under shadows.
"""

import math
import os
import sys

from fontTools.colorLib.builder import buildCOLR, buildCPAL
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib.tables.otTables import PaintFormat
from shapely.geometry import LineString, Point
from shapely.geometry.polygon import orient
from shapely.ops import unary_union

FAMILY = "OyNIx Neu"
VERSION = "1.000"
UPM = 1000
STROKE = 92
H = STROKE / 2

# Vertical metrics (outline edges) and the skeleton lines inside them.
CAP, XH, ASC, DESC = 700, 520, 740, -200
B = H              # baseline skeleton
C = CAP - H        # cap height skeleton
X = XH - H         # x-height skeleton
A = ASC - H        # ascender skeleton
D = DESC + H       # descender skeleton
SB = 64            # side bearing (outline to advance edge)


# ── Skeleton helpers ────────────────────────────────────────────────

def arc(cx, cy, rx, ry, a0, a1, steps=None):
    """Points along an ellipse from angle a0 to a1 (degrees, counter-
    clockwise from +x; a1 < a0 goes clockwise)."""
    n = steps or max(8, int(abs(a1 - a0) / 6))
    return [(cx + rx * math.cos(math.radians(a0 + (a1 - a0) * i / n)),
             cy + ry * math.sin(math.radians(a0 + (a1 - a0) * i / n))) for i in range(n + 1)]


def ring(cx, cy, rx, ry):
    return arc(cx, cy, rx, ry, 90, 450, 72)


def bowl_r(x0, top, bot, w):
    """A D-shaped bowl opening left at x0: top bar, round right side, bottom bar."""
    r = (top - bot) / 2
    xe = x0 + w - r
    return [(x0, top), (xe, top)] + arc(xe, (top + bot) / 2, r, r, 90, -90) + [(x0, bot)]


class G:
    """A glyph under construction: strokes, dots and its advance width."""

    def __init__(self, width):
        self.w = width          # skeleton box width
        self.strokes = []
        self.dots = []

    @property
    def l(self):
        return SB + H

    @property
    def r(self):
        return SB + H + self.w

    @property
    def m(self):
        return SB + H + self.w / 2

    def s(self, *pts):
        pts = [p for p in pts]
        flat = []
        for p in pts:
            if isinstance(p, list):
                flat.extend(p)
            else:
                flat.append(p)
        self.strokes.append(flat)
        return self

    def dot(self, x, y, scale=1.12):
        self.dots.append((x, y, H * scale))
        return self

    @property
    def advance(self):
        return round(self.w + STROKE + 2 * SB)


GLYPHS = {}


def glyph(ch, width):
    g = G(width)
    GLYPHS[ch] = g
    return g


def caps():
    MID = (B + C) / 2 + 10
    g = glyph("A", 520); g.s((g.l, B), (g.m, C), (g.r, B)); g.s((g.l + 95, 230), (g.r - 95, 230))
    g = glyph("B", 400); g.s((g.l, B), (g.l, C)); g.s(bowl_r(g.l, C, MID, g.w - 30)); g.s(bowl_r(g.l, MID, B, g.w))
    g = glyph("C", 500); g.s(arc(g.m, (B + C) / 2, g.w / 2, (C - B) / 2, 42, 318))
    g = glyph("D", 470); g.s((g.l, B), (g.l, C)); g.s(bowl_r(g.l, C, B, g.w))
    g = glyph("E", 380); g.s((g.r, C), (g.l, C), (g.l, B), (g.r, B)); g.s((g.l, MID), (g.r - 50, MID))
    g = glyph("F", 370); g.s((g.r, C), (g.l, C), (g.l, B)); g.s((g.l, MID), (g.r - 50, MID))
    g = glyph("G", 520)
    cy = (B + C) / 2
    g.s(arc(g.m, cy, g.w / 2, (C - B) / 2, 42, 360) + [(g.r, cy), (g.m + 40, cy)])
    g = glyph("H", 460); g.s((g.l, B), (g.l, C)); g.s((g.r, B), (g.r, C)); g.s((g.l, MID), (g.r, MID))
    g = glyph("I", 0); g.s((g.m, B), (g.m, C))
    g = glyph("J", 330); rr = 150
    g.s([(g.r, C), (g.r, B + rr)] + arc(g.r - rr, B + rr, rr, rr, 0, -180))
    g = glyph("K", 430); g.s((g.l, B), (g.l, C)); g.s((g.r, C), (g.l, 250)); g.s((g.l + 140, 370), (g.r, B))
    g = glyph("L", 360); g.s((g.l, C), (g.l, B), (g.r, B))
    g = glyph("M", 600); g.s((g.l, B), (g.l, C), (g.m, 230), (g.r, C), (g.r, B))
    g = glyph("N", 460); g.s((g.l, B), (g.l, C), (g.r, B), (g.r, C))
    g = glyph("O", 580); g.s(ring(g.m, (B + C) / 2, g.w / 2, (C - B) / 2))
    g = glyph("P", 400); g.s((g.l, B), (g.l, C)); g.s(bowl_r(g.l, C, 300, g.w))
    g = glyph("Q", 580); g.s(ring(g.m, (B + C) / 2, g.w / 2, (C - B) / 2)); g.s((g.m + 90, 190), (g.r + 30, B - 20))
    g = glyph("R", 420); g.s((g.l, B), (g.l, C)); g.s(bowl_r(g.l, C, 320, g.w - 10)); g.s((g.l + 170, 320), (g.r, B))
    g = glyph("S", 400)
    ry1 = (C - MID) / 2; ry2 = (MID - B) / 2
    g.s(arc(g.m, MID + ry1, g.w / 2 - 10, ry1, 25, 270) + arc(g.m, MID - ry2, g.w / 2, ry2, 90, -155))
    g = glyph("T", 460); g.s((g.l, C), (g.r, C)); g.s((g.m, C), (g.m, B))
    g = glyph("U", 450); rr = g.w / 2
    g.s([(g.l, C), (g.l, B + rr)] + arc(g.m, B + rr, rr, rr, 180, 360) + [(g.r, C)])
    g = glyph("V", 500); g.s((g.l, C), (g.m, B), (g.r, C))
    g = glyph("W", 700); g.s((g.l, C), (g.l + g.w * 0.24, B), (g.m, C - 120), (g.r - g.w * 0.24, B), (g.r, C))
    g = glyph("X", 470); g.s((g.l, C), (g.r, B)); g.s((g.r, C), (g.l, B))
    g = glyph("Y", 480); g.s((g.l, C), (g.m, 330)); g.s((g.r, C), (g.m, 330), (g.m, B))
    g = glyph("Z", 420); g.s((g.l, C), (g.r, C), (g.l, B), (g.r, B))


def lower():
    cy = (B + X) / 2
    ry = (X - B) / 2

    def bowl(g, cx=None, rx=None):
        g.s(ring(cx if cx is not None else g.m, cy, rx if rx is not None else g.w / 2, ry))

    g = glyph("a", 400); bowl(g); g.s((g.r, X), (g.r, B))
    g = glyph("b", 400); bowl(g); g.s((g.l, A), (g.l, B))
    g = glyph("c", 380); g.s(arc(g.m, cy, g.w / 2, ry, 45, 315))
    g = glyph("d", 400); bowl(g); g.s((g.r, A), (g.r, B))
    g = glyph("e", 400); g.s([(g.l + 10, cy), (g.r, cy)] + arc(g.m, cy, g.w / 2, ry, 0, 315))
    g = glyph("f", 250); rr = 110
    g.s([(g.l + 60, B), (g.l + 60, A - rr)] + arc(g.l + 60 + rr, A - rr, rr, rr, 180, 60))
    g.s((g.l - 20, X), (g.r - 20, X))
    g = glyph("g", 400); bowl(g); rr = 150
    g.s([(g.r, X), (g.r, D + rr)] + arc(g.r - rr, D + rr, rr, rr, 0, -160))
    g = glyph("h", 380); rr = g.w / 2
    g.s((g.l, A), (g.l, B)); g.s([(g.l, X - rr)] + arc(g.m, X - rr, rr, rr, 180, 0) + [(g.r, B)])
    g = glyph("i", 0); g.s((g.m, B), (g.m, X)); g.dot(g.m, X + 160)
    g = glyph("j", 120); rr = 120
    g.s([(g.r, X), (g.r, D + rr)] + arc(g.r - rr, D + rr, rr, rr, 0, -150)); g.dot(g.r, X + 160)
    g = glyph("k", 370); g.s((g.l, A), (g.l, B)); g.s((g.r, X), (g.l, 190)); g.s((g.l + 120, 290), (g.r, B))
    g = glyph("l", 0); g.s((g.m, A), (g.m, B))
    g = glyph("m", 600); rr = g.w / 4
    g.s((g.l, X), (g.l, B))
    g.s([(g.l, X - rr)] + arc(g.l + rr, X - rr, rr, rr, 180, 0) + [(g.m, B)])
    g.s([(g.m, X - rr)] + arc(g.m + rr, X - rr, rr, rr, 180, 0) + [(g.r, B)])
    g = glyph("n", 380); rr = g.w / 2
    g.s((g.l, X), (g.l, B)); g.s([(g.l, X - rr)] + arc(g.m, X - rr, rr, rr, 180, 0) + [(g.r, B)])
    g = glyph("o", 420); bowl(g)
    g = glyph("p", 400); bowl(g); g.s((g.l, X), (g.l, D))
    g = glyph("q", 400); bowl(g); g.s((g.r, X), (g.r, D))
    g = glyph("r", 170); rr = 140
    g.s((g.l, X), (g.l, B)); g.s([(g.l, X - rr)] + arc(g.l + rr, X - rr, rr, rr, 180, 70))
    g = glyph("s", 330)
    mid = cy + 6; ry1 = (X - mid) / 2; ry2 = (mid - B) / 2
    g.s(arc(g.m, mid + ry1, g.w / 2 - 10, ry1, 25, 270) + arc(g.m, mid - ry2, g.w / 2, ry2, 90, -155))
    g = glyph("t", 250); rr = 100
    g.s([(g.l + 60, A - 60), (g.l + 60, B + rr)] + arc(g.l + 60 + rr, B + rr, rr, rr, 180, 285))
    g.s((g.l - 20, X), (g.r - 10, X))
    g = glyph("u", 380); rr = g.w / 2
    g.s([(g.l, X), (g.l, B + rr)] + arc(g.m, B + rr, rr, rr, 180, 360)); g.s((g.r, X), (g.r, B))
    g = glyph("v", 400); g.s((g.l, X), (g.m, B), (g.r, X))
    g = glyph("w", 600); g.s((g.l, X), (g.l + g.w * 0.24, B), (g.m, X - 80), (g.r - g.w * 0.24, B), (g.r, X))
    g = glyph("x", 380); g.s((g.l, X), (g.r, B)); g.s((g.r, X), (g.l, B))
    g = glyph("y", 400); g.s((g.l, X), (g.m, B)); g.s((g.r, X), (g.m - 70, D))
    g = glyph("z", 350); g.s((g.l, X), (g.r, X), (g.l, B), (g.r, B))


def digits():
    cy = (B + C) / 2
    g = glyph("0", 420); g.s(ring(g.m, cy, g.w / 2, (C - B) / 2))
    g = glyph("1", 220); g.s((g.l, C - 110), (g.r, C), (g.r, B))
    g = glyph("2", 400); r = g.w / 2
    g.s(arc(g.m, C - r + 20, r, r - 20, 165, -20) + [(g.l, B), (g.r, B)])
    g = glyph("3", 390); mid = cy + 20
    g.s(arc(g.m - 10, (C + mid) / 2, g.w / 2 - 10, (C - mid) / 2, 150, -90))
    g.s(arc(g.m, (mid + B) / 2, g.w / 2, (mid - B) / 2, 90, -150))
    g = glyph("4", 430); g.s((g.r - 90, B), (g.r - 90, C), (g.l, 220), (g.r, 220))
    g = glyph("5", 400); top = 420
    g.s([(g.r - 10, C), (g.l + 20, C), (g.l, top)] + arc(g.m, (top + B) / 2 + 4, g.w / 2, (top - B) / 2 + 4, 140, -150))
    g = glyph("6", 410); r2 = (380 - B) / 2
    g.s(ring(g.m, B + r2, g.w / 2, r2)); g.s(arc(g.m + 40, B + r2 + 10, g.w / 2 + 40, C - B - r2 - 10, 75, 175))
    g = glyph("7", 390); g.s((g.l, C), (g.r, C), (g.m - 30, B))
    g = glyph("8", 410); mid = cy + 30
    g.s(ring(g.m, (C + mid) / 2, g.w / 2 - 20, (C - mid) / 2)); g.s(ring(g.m, (mid + B) / 2, g.w / 2, (mid - B) / 2))
    g = glyph("9", 410); r2 = (C - 320) / 2
    g.s(ring(g.m, C - r2, g.w / 2, r2)); g.s(arc(g.m - 40, C - r2 - 10, g.w / 2 + 40, C - B - r2 - 10, 255, 355))


def punctuation():
    g = glyph(".", 0); g.dot(g.m, B, 1.15)
    g = glyph(",", 40); g.dot(g.l, B, 1.15); g.s((g.l + 18, B), (g.l - 40, B - 150))
    g = glyph(":", 0); g.dot(g.m, B, 1.15); g.dot(g.m, X - 20, 1.15)
    g = glyph(";", 40); g.dot(g.l, X - 20, 1.15); g.dot(g.l, B, 1.15); g.s((g.l + 18, B), (g.l - 40, B - 150))
    g = glyph("!", 0); g.s((g.m, C), (g.m, 230)); g.dot(g.m, B, 1.15)
    g = glyph("?", 340); r = g.w / 2
    g.s(arc(g.m, C - r + 10, r, r - 10, 160, -60) + [(g.m, 320), (g.m, 240)]); g.dot(g.m, B, 1.15)
    g = glyph("'", 0); g.s((g.m, C), (g.m, C - 170))
    g = glyph('"', 120); g.s((g.l, C), (g.l, C - 170)); g.s((g.r, C), (g.r, C - 170))
    g = glyph("-", 240); g.s((g.l, 290), (g.r, 290))
    g = glyph("_", 420); g.s((g.l, D + 60), (g.r, D + 60))
    g = glyph("/", 320); g.s((g.l, D + 40), (g.r, A))
    g = glyph("\\", 320); g.s((g.l, A), (g.r, D + 40))
    g = glyph("(", 150); g.s(arc(g.r + 60, (A + D) / 2, 210, (A - D) / 2 + 30, 118, 242))
    g = glyph(")", 150); g.s(arc(g.l - 60, (A + D) / 2, 210, (A - D) / 2 + 30, 62, -62))
    g = glyph("+", 360); g.s((g.l, 300), (g.r, 300)); g.s((g.m, 120), (g.m, 480))
    g = glyph("=", 360); g.s((g.l, 380), (g.r, 380)); g.s((g.l, 220), (g.r, 220))
    g = glyph("&", 470)
    g.s([(g.r, B)] + [(g.l + 140, 420)] + arc(g.l + 170, 545, 120, 110, 220, -30) + [(g.l + 40, 230)]
        + arc(g.l + 170, 190, 160, 145, 160, 330) + [(g.r, 330)])
    g = glyph("#", 440); g.s((g.l + 120, B + 20), (g.l + 190, C - 20)); g.s((g.r - 190, B + 20), (g.r - 120, C - 20))
    g.s((g.l, 440), (g.r, 440)); g.s((g.l, 260), (g.r, 260))
    g = glyph("%", 560)
    g.s(ring(g.l + 90, C - 100, 90, 100)); g.s(ring(g.r - 90, B + 100, 90, 100)); g.s((g.l, B), (g.r, C))
    g = glyph("@", 640)
    cy = (B + C) / 2 - 20
    g.s(ring(g.m + 10, cy, 110, 130)); g.s([(g.m + 120, cy + 130), (g.m + 120, cy - 70)]
        + arc(g.m + 190, cy - 70, 70, 70, 180, 360) + arc(g.m, cy, g.w / 2, (C - B) / 2 + 10, 0, 300))
    g = glyph("*", 320); cx, cyy = g.m, C - 140
    for ang in (90, 30, 150):
        dx, dy = 140 * math.cos(math.radians(ang)), 140 * math.sin(math.radians(ang))
        g.s((cx - dx, cyy - dy), (cx + dx, cyy + dy))
    g = glyph("<", 340); g.s((g.r, 480), (g.l, 300), (g.r, 120))
    g = glyph(">", 340); g.s((g.l, 480), (g.r, 300), (g.l, 120))
    g = glyph("[", 160); g.s((g.r, A), (g.l, A), (g.l, D), (g.r, D))
    g = glyph("]", 160); g.s((g.l, A), (g.r, A), (g.r, D), (g.l, D))
    g = glyph("|", 0); g.s((g.m, A), (g.m, D))
    g = glyph("~", 380); g.s(arc(g.l + 95, 300, 95, 50, 180, 0) + arc(g.r - 95, 300, 95, 50, 180, 360))
    g = glyph("$", 400)
    MID = (B + C) / 2 + 10
    ry1 = (C - MID) / 2 - 20; ry2 = (MID - B) / 2 - 20
    g.s(arc(g.m, MID + ry1, g.w / 2 - 10, ry1, 25, 270) + arc(g.m, MID - ry2, g.w / 2, ry2, 90, -155))
    g.s((g.m, C + 40), (g.m, B - 40))
    g = glyph("·", 0); g.dot(g.m, 300, 1.15)
    g = glyph("…", 520); [g.dot(g.l + i * 260, B, 1.15) for i in range(3)]
    g = glyph("–", 420); g.s((g.l, 290), (g.r, 290))
    g = glyph("—", 820); g.s((g.l, 290), (g.r, 290))
    g = glyph("‘", 0); g.s((g.m, C - 170), (g.m, C))
    g = glyph("’", 0); g.s((g.m, C), (g.m, C - 170))
    g = glyph("“", 120); g.s((g.l, C - 170), (g.l, C)); g.s((g.r, C - 170), (g.r, C))
    g = glyph("”", 120); g.s((g.l, C), (g.l, C - 170)); g.s((g.r, C), (g.r, C - 170))
    g = glyph("×", 330); g.s((g.l, 140), (g.r, 460)); g.s((g.l, 460), (g.r, 140))
    g = glyph("•", 0); g.dot(g.m, 300, 2.0)


caps()
lower()
digits()
punctuation()


# ── Outlines ────────────────────────────────────────────────────────

def outline(g):
    """The glyph's filled shape: every stroke buffered, unioned."""
    parts = [LineString(s).buffer(H, quad_segs=10, cap_style="round", join_style="round") for s in g.strokes if len(s) > 1]
    parts += [Point(x, y).buffer(r, quad_segs=10) for x, y, r in g.dots]
    if not parts:
        return None
    return unary_union(parts).simplify(0.8, preserve_topology=True)


def draw(shape, pen):
    polys = [shape] if shape.geom_type == "Polygon" else list(shape.geoms)
    for p in polys:
        # TrueType: outer contours clockwise, holes counter-clockwise.
        p = orient(p, sign=-1.0)
        for ringc in [p.exterior] + list(p.interiors):
            pts = [(round(x), round(y)) for x, y in ringc.coords[:-1]]
            dedup = [pt for i, pt in enumerate(pts) if pt != pts[i - 1]]
            if len(dedup) < 3:
                continue
            pen.moveTo(dedup[0])
            for pt in dedup[1:]:
                pen.lineTo(pt)
            pen.closePath()


def name_of(ch):
    if ch == " ":
        return "space"
    from fontTools.agl import UV2AGL
    return UV2AGL.get(ord(ch), "uni%04X" % ord(ch))


def build(color):
    order = [".notdef", "space"] + [name_of(c) for c in GLYPHS]
    cmap = {32: "space", 0xA0: "space"}
    glyf, metrics = {}, {}

    pen = TTGlyphPen(None)
    for x0, y0, x1, y1 in [(60, 0, 440, CAP), (130, 70, 370, CAP - 70)]:
        pts = [(x0, y0), (x0, y1), (x1, y1), (x1, y0)]
        if x0 == 130:
            pts.reverse()
        pen.moveTo(pts[0]); [pen.lineTo(p) for p in pts[1:]]; pen.closePath()
    glyf[".notdef"] = pen.glyph(); metrics[".notdef"] = (500, 60)
    glyf["space"] = TTGlyphPen(None).glyph(); metrics["space"] = (300, 0)

    bounds = {}
    for ch, g in GLYPHS.items():
        n = name_of(ch)
        cmap[ord(ch)] = n
        pen = TTGlyphPen(None)
        shape = outline(g)
        if shape is not None:
            draw(shape, pen)
            bounds[n] = shape.bounds
        glyf[n] = pen.glyph()
        lsb = round(shape.bounds[0]) if shape is not None else 0
        metrics[n] = (g.advance, lsb)

    style = "Color" if color else "Regular"
    fb = FontBuilder(UPM, isTTF=True)
    fb.setupGlyphOrder(order)
    fb.setupCharacterMap(cmap)
    fb.setupGlyf(glyf)
    fb.setupHorizontalMetrics(metrics)
    fb.setupHorizontalHeader(ascent=ASC + 60, descent=DESC - 40)
    family = FAMILY + (" Color" if color else "")
    fb.setupNameTable({
        "familyName": family,
        "styleName": "Regular",
        "uniqueFontIdentifier": f"OyNIx:{family.replace(' ', '')}-Regular:{VERSION}",
        "fullName": family,
        "psName": family.replace(" ", "") + "-Regular",
        "version": f"Version {VERSION}",
        "copyright": "Copyright 2026 the OyNIx authors",
        "licenseDescription": "MIT License",
        "designer": "OyNIx",
        "description": "A soft, round monoline for OyNIx's neumorphic styles" + (", with COLRv1 depth, shadow and light." if color else "."),
    })
    fb.setupOS2(sTypoAscender=ASC + 60, sTypoDescender=DESC - 40, sTypoLineGap=120, usWinAscent=ASC + 120, usWinDescent=-(DESC - 80),
                sxHeight=XH, sCapHeight=CAP, achVendID="OYNX", usWeightClass=400, fsSelection=0x40 | 0x80, version=4)
    fb.setupPost()
    fb.setupHead(unitsPerEm=UPM, fontRevision=float(VERSION))

    if color:
        add_colr(fb, bounds)
    fb.font["head"].flags |= 1 << 3  # integer ppem scaling
    return fb.font


# ── COLRv1: the 2.5D look baked into each glyph ─────────────────────

# Palette 0: light (Neumorphic), palette 1: dark (Graphite).
PALETTES = [
    # shadow, extrusion, rim light, face top, face bottom
    [(0.36, 0.40, 0.48, 0.16), (0.70, 0.73, 0.79, 1.0), (1.0, 1.0, 1.0, 0.95), (0.40, 0.43, 0.50, 1.0), (0.22, 0.24, 0.29, 1.0)],
    [(0.0, 0.0, 0.0, 0.30), (0.10, 0.11, 0.13, 1.0), (1.0, 1.0, 1.0, 0.16), (0.90, 0.92, 0.95, 1.0), (0.66, 0.70, 0.76, 1.0)],
]
DEPTH = 34      # extrusion, font units (down-right)
SHADOW = 56     # soft shadow distance
RIM = 18        # light rim (up-left)


def add_colr(fb, bounds):
    def layer(glyph, paint, dx=0, dy=0):
        p = {"Format": PaintFormat.PaintGlyph, "Glyph": glyph, "Paint": paint}
        if dx or dy:
            p = {"Format": PaintFormat.PaintTranslate, "dx": dx, "dy": dy, "Paint": p}
        return p

    def solid(i, alpha=1.0):
        return {"Format": PaintFormat.PaintSolid, "PaletteIndex": i, "Alpha": alpha}

    base = {}
    clips = {}
    for name, (x0, y0, x1, y1) in bounds.items():
        layers = []
        # A soft shadow: a few faint, growing copies further down-right.
        for k in range(4):
            t = SHADOW * (0.55 + 0.15 * k)
            layers.append(layer(name, solid(0, 1.0), t, -t))
        # The extruded side: copies stepping toward the face.
        steps = 6
        for k in range(steps, 0, -1):
            t = DEPTH * k / steps
            layers.append(layer(name, solid(1), t, -t))
        # The rim light, up-left.
        layers.append(layer(name, solid(2), -RIM, RIM))
        # The face: lit from the top-left.
        grad = {
            "Format": PaintFormat.PaintLinearGradient,
            "ColorLine": {"Extend": "pad", "ColorStop": [{"StopOffset": 0.0, "PaletteIndex": 3, "Alpha": 1.0},
                                                         {"StopOffset": 1.0, "PaletteIndex": 4, "Alpha": 1.0}]},
            "x0": round(x0), "y0": round(y1), "x1": round(x1 * 0.3 + x0 * 0.7), "y1": round(y0),
            "x2": round(x1), "y2": round(y1),
        }
        layers.append(layer(name, grad))
        base[name] = (PaintFormat.PaintColrLayers, layers)
        pad = SHADOW * 1.0 + 10
        clips[name] = (round(x0 - RIM - 4), round(y0 - pad), round(x1 + pad), round(y1 + RIM + 4))
    fb.font["COLR"] = buildCOLR(base, version=1, glyphMap=fb.font.getReverseGlyphMap(), clipBoxes=clips)
    fb.font["CPAL"] = buildCPAL(PALETTES)


def main():
    out = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "fonts")
    os.makedirs(out, exist_ok=True)
    for color, file in [(False, "OyNIxNeu-Regular.ttf"), (True, "OyNIxNeu-Color.ttf")]:
        f = build(color)
        path = os.path.join(out, file)
        f.save(path)
        print(f"{path}: {len(GLYPHS)} characters")


if __name__ == "__main__":
    sys.exit(main())
