#version 440
// OyNIx Neu text: the neumorphic, slightly 2.5D look of headings in the
// Neumorphic and Graphite styles, drawn in one pass from the text's own
// alpha. Back to front: a soft shadow down-right, the extruded side (the
// text stepped toward the viewer), a light rim up-left, then the face,
// lit from the top-left.
//
// The effect area is the text plus `pad` on every side; `source` is the
// text alone (premultiplied, `srcSize` pixels).

layout(location = 0) in vec2 qt_TexCoord0;
layout(location = 0) out vec4 fragColor;

layout(std140, binding = 0) uniform buf {
    mat4 qt_Matrix;
    float qt_Opacity;
    vec2 srcSize;    // the text, pixels
    float pad;       // margin around it, pixels
    float depth;     // extrusion, pixels (down-right)
    float blur;      // shadow softness, pixels
    float shadowDist; // shadow offset, pixels
    vec4 shadow;     // straight alpha
    vec4 side;       // extrusion colour, straight alpha
    vec4 rimLight;   // rim colour, straight alpha
    float lift;      // face brightness change, top to bottom (e.g. 0.18)
};
layout(binding = 1) uniform sampler2D source;

float alphaAt(vec2 px) {
    vec2 uv = px / srcSize;
    if (uv.x < 0.0 || uv.y < 0.0 || uv.x > 1.0 || uv.y > 1.0) return 0.0;
    return texture(source, uv).a;
}

vec4 over(vec4 top, vec4 under) { return top + under * (1.0 - top.a); }
vec4 premul(vec4 c, float a) { return vec4(c.rgb * c.a * a, c.a * a); }

void main() {
    vec2 px = qt_TexCoord0 * (srcSize + vec2(2.0 * pad)) - vec2(pad);

    // Soft shadow: a 5×5 blur of the text, moved down-right.
    float sh = 0.0;
    for (int y = -2; y <= 2; y++)
        for (int x = -2; x <= 2; x++)
            sh += alphaAt(px - vec2(shadowDist) + vec2(float(x), float(y)) * blur * 0.5);
    sh /= 25.0;

    // The extruded side: the text repeated toward down-right.
    float ex = 0.0;
    for (int i = 1; i <= 6; i++)
        ex = max(ex, alphaAt(px - vec2(depth * float(i) / 6.0)));

    // Rim light: the text moved up-left.
    float rim = alphaAt(px + vec2(max(1.0, depth * 0.35)));

    vec4 col = premul(shadow, sh);
    col = over(premul(side, ex), col);
    col = over(premul(rimLight, rim), col);

    // The face, lit from the top-left.
    vec2 uv = px / srcSize;
    vec4 face = vec4(0.0);
    if (uv.x >= 0.0 && uv.y >= 0.0 && uv.x <= 1.0 && uv.y <= 1.0) {
        face = texture(source, uv);
        // Lighter toward the top-left, darker toward the bottom-right
        // (added, so dark text gets the gradient too).
        float t = uv.y * 0.8 + uv.x * 0.2;
        face.rgb = clamp(face.rgb + vec3(lift * (0.5 - t)) * face.a, vec3(0.0), vec3(face.a));
    }
    fragColor = over(face, col) * qt_Opacity;
}
