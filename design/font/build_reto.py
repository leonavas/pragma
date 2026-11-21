"""Build Pragma Reto, the display font of pragma.

Every glyph is a set of rectangles on a grid: stroke 3, x-height 16, cap height 22,
descender 8. There are no curves and no diagonals.

Run:  uv run --with fonttools --with brotli design/font/build_reto.py
Out:  site/public/fonts/pragma-reto.woff2 and design/font/pragma-reto.ttf
"""

from pathlib import Path

from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen

UNIT = 32
UPM = 1024
S = 3
X = 16
C = 22
D = -8
GAP = 6

ROOT = Path(__file__).resolve().parents[2]


def v(x, y0, y1):
    return (x, y0, x + S, y1)


def h(y, x0, x1):
    return (x0, y, x1, y + S)


def ring(w, top):
    return [h(top - S, 0, w), h(0, 0, w), v(0, 0, top), v(w - S, 0, top)]


def stairs(w, top, block_w, block_h):
    step_x = (w - block_w) / 4
    step_y = (top - block_h) / 4
    return [(step_x * i, step_y * i, step_x * i + block_w, step_y * i + block_h) for i in range(5)]


def mirror(rects, w):
    return [(w - x1, y0, w - x0, y1) for (x0, y0, x1, y1) in rects]


def flip(rects, top):
    return [(x0, top - y1, x1, top - y0) for (x0, y0, x1, y1) in rects]


def glyphs(W):
    R = W - S
    N = W - 2
    NR = N - S
    WIDE = W + 6
    WM = (WIDE - S) / 2
    XM = (X - S) / 2
    CM = (C - S) / 2
    DOT = (0, C - S, S, C)

    def five(w, top, mid):
        r = w - S
        return [h(top - S, 0, w), v(0, mid, top), h(mid, 0, w), v(r, 0, mid + S), h(0, 0, w)]

    def two(w, top, mid):
        return mirror(five(w, top, mid), w)

    def vee(w, top):
        low = top - 10 if top == X else top - 14
        return [
            (0, low, S, top), (w - S, low, w, top),
            (S, low - 3, 2 * S, low + 3), (w - 2 * S, low - 3, w - S, low + 3),
            (2 * S, 0, w - 2 * S, low),
        ]

    def arms(top, block_h):
        rising = stairs(W, top, 4, block_h)
        return [rect for rect in rising + flip(rising, top) if rect[0] >= 6]

    lower = {
        'a': (W, [h(X - S, 0, W), v(R, 0, X), h(XM, 0, W), v(0, 0, XM + S), h(0, 0, W)]),
        'b': (W, [v(0, 0, C)] + ring(W, X)),
        'c': (N, [h(X - S, 0, N), v(0, 0, X), h(0, 0, N)]),
        'd': (W, [v(R, 0, C)] + ring(W, X)),
        'e': (W, [h(X - S, 0, W), v(0, 0, X), v(R, XM, X), h(XM, 0, W), h(0, 0, W)]),
        'f': (11, [v(0, 0, C), h(C - S, 0, 11), h(X - S, 0, 10)]),
        'g': (W, ring(W, X) + [v(R, D, X), h(D, 0, W)]),
        'h': (W, [v(0, 0, C), h(X - S, 0, W), v(R, 0, X)]),
        'i': (S, [v(0, 0, X), DOT]),
        'j': (8, [v(5, D, X), h(D, 0, 8), (5, C - S, 8, C)]),
        'k': (W, [v(0, 0, C), h(XM, S, 6)] + arms(X, 4)),
        'l': (8, [v(0, 0, C), h(0, 0, 8)]),
        'm': (WIDE, [h(X - S, 0, WIDE), v(0, 0, X), v(WM, 0, X), v(WIDE - S, 0, X)]),
        'n': (W, [h(X - S, 0, W), v(0, 0, X), v(R, 0, X)]),
        'o': (W, ring(W, X)),
        'p': (W, ring(W, X) + [v(0, D, X)]),
        'q': (W, ring(W, X) + [v(R, D, X)]),
        'r': (11, [v(0, 0, X), h(X - S, 0, 11)]),
        's': (N, five(N, X, XM)),
        't': (10, [v(0, 0, C - 1), h(X - S, 0, 10), h(0, 0, 10)]),
        'u': (W, [v(0, 0, X), v(R, 0, X), h(0, 0, W)]),
        'v': (W, vee(W, X)),
        'w': (WIDE, [h(0, 0, WIDE), v(0, 0, X), v(WM, 0, X), v(WIDE - S, 0, X)]),
        'x': (W, stairs(W, X, 4, 4) + mirror(stairs(W, X, 4, 4), W)),
        'y': (W, [v(0, 0, X), h(0, 0, W), v(R, D, X), h(D, 0, W)]),
        'z': (N, two(N, X, XM)),
    }

    upper = {
        'A': (W, [h(C - S, 0, W), v(0, 0, C), v(R, 0, C), h(CM, 0, W)]),
        'B': (W, [v(0, 0, C), h(C - S, 0, R), v(R - S, CM, C), h(CM, 0, W), v(R, 0, CM + S), h(0, 0, W)]),
        'C': (W - 1, [h(C - S, 0, W - 1), v(0, 0, C), h(0, 0, W - 1)]),
        'D': (W, [v(0, 0, C), h(C - S, 0, W - 1.5), h(0, 0, W - 1.5), v(R, 1.5, C - 1.5)]),
        'E': (W - 1, [v(0, 0, C), h(C - S, 0, W - 1), h(CM, 0, W - 3), h(0, 0, W - 1)]),
        'F': (W - 1, [v(0, 0, C), h(C - S, 0, W - 1), h(CM, 0, W - 3)]),
        'G': (W, [h(C - S, 0, W), v(0, 0, C), h(0, 0, W), v(R, 0, CM + S), h(CM, W / 2, W)]),
        'H': (W, [v(0, 0, C), v(R, 0, C), h(CM, 0, W)]),
        'I': (11, [v(4, 0, C), h(C - S, 0, 11), h(0, 0, 11)]),
        'J': (W, [v(R, 0, C), h(0, 0, W), v(0, 0, 8)]),
        'K': (W, [v(0, 0, C), h(CM, S, 6)] + arms(C, 6)),
        'L': (W - 1, [v(0, 0, C), h(0, 0, W - 1)]),
        'M': (WIDE, [h(C - S, 0, WIDE), v(0, 0, C), v(WM, 0, C), v(WIDE - S, 0, C)]),
        'N': (W, [h(C - S, 0, W), v(0, 0, C), v(R, 0, C)]),
        'O': (W, ring(W, C)),
        'P': (W, [v(0, 0, C), h(C - S, 0, W), v(R, CM, C), h(CM, 0, W)]),
        'Q': (W, ring(W, C) + [(R - S, -4, R, S)]),
        'R': (W, [v(0, 0, C), h(C - S, 0, R), v(R - S, CM, C), h(CM, 0, W), v(R, 0, CM + S)]),
        'S': (W, five(W, C, CM)),
        'T': (W, [h(C - S, 0, W), v((W - S) / 2, 0, C)]),
        'U': (W, [v(0, 0, C), v(R, 0, C), h(0, 0, W)]),
        'V': (W, vee(W, C)),
        'W': (WIDE, [h(0, 0, WIDE), v(0, 0, C), v(WM, 0, C), v(WIDE - S, 0, C)]),
        'X': (W, stairs(W, C, 4, 6) + mirror(stairs(W, C, 4, 6), W)),
        'Y': (W, [v(0, CM, C), v(R, CM, C), h(CM, 0, W), v((W - S) / 2, 0, CM + S)]),
        'Z': (W, two(W, C, CM)),
    }

    NM = (N - S) / 2
    digits = {
        '0': (N, ring(N, C) + [(NM, CM, NM + S, CM + S)]),
        '1': (N, [v(NM, 0, C), h(C - S, 1, NM + S), h(0, 1, N - 1)]),
        '2': (N, two(N, C, CM)),
        '3': (N, [h(C - S, 0, N), h(CM, 2, N), h(0, 0, N), v(NR, 0, C)]),
        '4': (N, [v(0, CM, C), h(CM, 0, N), v(NR, 0, C)]),
        '5': (N, five(N, C, CM)),
        '6': (N, [h(C - S, 0, N), v(0, 0, C), h(CM, 0, N), v(NR, 0, CM + S), h(0, 0, N)]),
        '7': (N, [h(C - S, 0, N), v(NR, 0, C)]),
        '8': (N, ring(N, C) + [h(CM, 0, N)]),
        '9': (N, [h(C - S, 0, N), v(0, CM, C), h(CM, 0, N), v(NR, 0, C), h(0, 0, N)]),
    }

    slash = [(3 * i, 5 * i, 3 * i + S, 5 * i + 7) for i in range(4)]
    paren = [(0, 1.5, S, C - 1.5), (1.5, C - S, 7, C), (1.5, 0, 7, S)]
    bracket = [v(0, 0, C), h(C - S, 0, 8), h(0, 0, 8)]
    marks = {
        '.': (S, [(0, 0, S, S)]),
        ',': (S, [(0, -4, S, S)]),
        ':': (S, [(0, 0, S, S), (0, 10, S, 13)]),
        ';': (S, [(0, -4, S, S), (0, 10, S, 13)]),
        '!': (S, [(0, 0, S, S), (0, 6.5, S, C)]),
        '?': (N, [h(C - S, 0, N), v(NR, CM, C), h(CM, NM, N), v(NM, 6.5, CM + S), (NM, 0, NM + S, S)]),
        "'": (S, [(0, 15, S, C)]),
        '"': (9, [(0, 15, S, C), (6, 15, 9, C)]),
        '-': (10, [h(XM, 0, 10)]),
        '_': (W, [(0, -4, W, -1)]),
        '/': (12, slash),
        '\\': (12, mirror(slash, 12)),
        '(': (7, paren),
        ')': (7, mirror(paren, 7)),
        '[': (8, bracket),
        ']': (8, mirror(bracket, 8)),
        '+': (13, [h(XM, 0, 13), v(5, 1.5, 14.5)]),
        '=': (13, [h(3.5, 0, 13), h(9.5, 0, 13)]),
        '|': (S, [(0, -4, S, C + 2)]),
        '#': (W, [v(3, 1, C - 1), v(W - 6, 1, C - 1), h(5, 0, W), h(14, 0, W)]),
    }

    extra = {
        '–': (14, [h(XM, 0, 14)]),
        '—': (WIDE, [h(XM, 0, WIDE)]),
        '·': (S, [(0, XM, S, XM + S)]),
        '’': marks["'"], '‘': marks["'"],
        '“': marks['"'], '”': marks['"'],
    }
    return {**lower, **upper, **digits, **marks, **extra}


SIDE = {
    'r': (3, 0.5), 'f': (3, 1), 't': (3, 1.5), 'c': (3, 2), 'L': (3, 1.5), 'F': (3, 1.5), 'T': (2, 2),
    '.': (2.5, 3), ',': (2.5, 3), ':': (3, 3), ';': (3, 3), "'": (2, 2), '"': (2, 2),
    '’': (2, 2), '‘': (2, 2), '“': (2, 2), '”': (2, 2),
    '/': (1.5, 1.5), '\\': (1.5, 1.5), '-': (3, 3),
}


def outline(rects):
    cells = set()
    for (x0, y0, x1, y1) in rects:
        for i in range(round(x0 * 2), round(x1 * 2)):
            for j in range(round(y0 * 2), round(y1 * 2)):
                cells.add((i, j))
    edges = {}
    for (i, j) in cells:
        if (i, j + 1) not in cells:
            edges.setdefault((i, j + 1), []).append((i + 1, j + 1))
        if (i + 1, j) not in cells:
            edges.setdefault((i + 1, j + 1), []).append((i + 1, j))
        if (i, j - 1) not in cells:
            edges.setdefault((i + 1, j), []).append((i, j))
        if (i - 1, j) not in cells:
            edges.setdefault((i, j), []).append((i, j + 1))
    contours = []
    while edges:
        start = next(iter(edges))
        point = start
        previous = None
        path = [start]
        while True:
            options = edges[point]
            if len(options) == 1 or previous is None:
                following = options[0]
            else:
                dx, dy = point[0] - previous[0], point[1] - previous[1]
                right = (point[0] + dy, point[1] - dx)
                following = right if right in options else options[0]
            options.remove(following)
            if not options:
                del edges[point]
            previous, point = point, following
            if point == start:
                break
            path.append(point)
        simple = []
        count = len(path)
        for index in range(count):
            before, here, after = path[index - 1], path[index], path[(index + 1) % count]
            if (here[0] - before[0], here[1] - before[1]) != (after[0] - here[0], after[1] - here[1]):
                simple.append(here)
        contours.append(simple)
    return contours


def build(width=X, family='Pragma Reto', out_name='pragma-reto'):
    table = glyphs(width)
    order = ['.notdef', 'space']
    cmap = {32: 'space', 160: 'space'}
    drawn = {}
    metrics = {'.notdef': (round((X + GAP) * UNIT), 3 * UNIT), 'space': (10 * UNIT, 0)}

    pen = TTGlyphPen(None)
    for contour in outline(ring(X, C)):
        scale = UNIT / 2
        pen.moveTo((contour[0][0] * scale + 3 * UNIT, contour[0][1] * scale))
        for (px, py) in contour[1:]:
            pen.lineTo((px * scale + 3 * UNIT, py * scale))
        pen.closePath()
    drawn['.notdef'] = pen.glyph()
    drawn['space'] = TTGlyphPen(None).glyph()

    for char, (glyph_width, rects) in table.items():
        name = f'uni{ord(char):04X}'
        left, right = SIDE.get(char, (3, 3))
        pen = TTGlyphPen(None)
        scale = UNIT / 2
        for contour in outline(rects):
            pen.moveTo((round(contour[0][0] * scale + left * UNIT), round(contour[0][1] * scale)))
            for (px, py) in contour[1:]:
                pen.lineTo((round(px * scale + left * UNIT), round(py * scale)))
            pen.closePath()
        drawn[name] = pen.glyph()
        order.append(name)
        cmap[ord(char)] = name
        metrics[name] = (round((left + glyph_width + right) * UNIT), round(left * UNIT))

    builder = FontBuilder(UPM, isTTF=True)
    builder.setupGlyphOrder(order)
    builder.setupCharacterMap(cmap)
    builder.setupGlyf(drawn)
    builder.setupHorizontalMetrics(metrics)
    ascent, descent = 26 * UNIT, -10 * UNIT
    builder.setupHorizontalHeader(ascent=ascent, descent=descent, lineGap=0)
    builder.setupNameTable({
        'familyName': family,
        'styleName': 'Regular',
        'uniqueFontIdentifier': f'{family} 1.0',
        'fullName': family,
        'psName': family.replace(' ', '') + '-Regular',
        'version': 'Version 1.000',
        'licenseDescription': 'MIT License. Part of pragma.',
    })
    builder.setupOS2(
        sTypoAscender=ascent, sTypoDescender=descent, sTypoLineGap=0,
        usWinAscent=ascent, usWinDescent=-descent,
        sxHeight=X * UNIT, sCapHeight=C * UNIT, usWeightClass=500, fsSelection=0x40,
    )
    builder.setupPost(underlinePosition=-3 * UNIT, underlineThickness=round(1.5 * UNIT))

    font = builder.font
    ttf_path = ROOT / 'design' / 'font' / f'{out_name}.ttf'
    font.save(ttf_path)
    font.flavor = 'woff2'
    woff_path = ROOT / 'site' / 'public' / 'fonts' / f'{out_name}.woff2'
    font.save(woff_path)
    print(f'{family}: {len(order)} glyphs, {woff_path.stat().st_size} bytes -> {woff_path.relative_to(ROOT)}')


if __name__ == '__main__':
    build()
