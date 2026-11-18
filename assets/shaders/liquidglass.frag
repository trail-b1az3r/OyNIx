#version 440
// OyNIx Liquid Glass: refracts the live content behind a rounded glass
// shape. Near the rim the content bends inward like a lens (with a little
// colour dispersion), the body is lightly frosted and tinted, and a
// specular rim is lit from the pointer's position.

layout(location = 0) in vec2 qt_TexCoord0;
layout(location = 0) out vec4 fragColor;

layout(std140, binding = 0) uniform buf {
    mat4 qt_Matrix;
    float qt_Opacity;
    vec2 size;      // the glass, in pixels
    float radius;   // corner radius, pixels
    float bend;     // refraction strength (pixels at the rim)
    float frost;    // blur radius, pixels
    vec2 pointer;   // pointer in glass coordinates; x < -1e3 if none
    vec4 tint;      // straight-alpha tint colour; .a = amount
    vec4 rim;       // rim colour; .a = brightness
};
layout(binding = 1) uniform sampler2D source; // what's behind, same area

float sdRoundRect(vec2 p, vec2 b, float r) {
    vec2 q = abs(p) - b + vec2(r);
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}

vec3 sampleFrost(vec2 uv, vec2 px) {
    // 9 taps on a small ring + centre: a cheap frost.
    vec3 c = texture(source, uv).rgb * 0.2;
    for (int i = 0; i < 8; i++) {
        float a = float(i) * 0.7853982;
        c += texture(source, uv + vec2(cos(a), sin(a)) * px * frost).rgb * 0.1;
    }
    return c;
}

void main() {
    vec2 p = qt_TexCoord0 * size;
    vec2 half_ = size * 0.5;
    float r = min(radius, min(half_.x, half_.y));
    float d = sdRoundRect(p - half_, half_, r);
    float inside = clamp(0.5 - d, 0.0, 1.0);           // anti-aliased mask
    if (inside <= 0.0) { fragColor = vec4(0.0); return; }

    // Surface normal from the distance field.
    vec2 e = vec2(1.0, 0.0);
    vec2 n = normalize(vec2(
        sdRoundRect(p + e.xy - half_, half_, r) - sdRoundRect(p - e.xy - half_, half_, r),
        sdRoundRect(p + e.yx - half_, half_, r) - sdRoundRect(p - e.yx - half_, half_, r)) + 1e-6);

    // Lens: strongest at the rim, fading over the edge band.
    float band = max(min(r, 22.0), 6.0);
    float t = clamp(1.0 + d / band, 0.0, 1.0);
    float lens = t * t * t;
    vec2 px = 1.0 / size;
    vec2 off = -n * lens * bend * px;

    // Dispersion: red bends least, blue most.
    vec3 col;
    col.r = sampleFrost(qt_TexCoord0 + off * 0.92, px).r;
    col.g = sampleFrost(qt_TexCoord0 + off * 1.00, px).g;
    col.b = sampleFrost(qt_TexCoord0 + off * 1.08, px).b;

    // Tint (the material's body colour).
    col = mix(col, tint.rgb, tint.a);

    // Specular rim, lit from the pointer (or the top-left).
    vec2 light = pointer.x < -1000.0 ? normalize(vec2(-0.6, -1.0)) : normalize(pointer - half_ + vec2(1e-3));
    float edge = smoothstep(2.5, 0.0, abs(d + 1.0));
    float facing = 0.35 + 0.65 * max(dot(n, light), 0.0);
    col += rim.rgb * rim.a * edge * facing;
    // A soft inner sheen along the lit side.
    col += rim.rgb * rim.a * 0.18 * lens * max(dot(n, light), 0.0);

    fragColor = vec4(col * inside, inside) * qt_Opacity;
}
