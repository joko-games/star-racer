// One gauge of the HUD: an arc of neon drawn in its node, thin, crisp and round-ended, with a tight
// glow around it, lit along part of its length over a hairline (see gauges.rs). Every number is
// a vec4, which WebGL2's uniform rules want.

#import bevy_ui::ui_vertex_output::UiVertexOutput

struct Arc {
    // Start and signed sweep of the arc, in radians clockwise from the right (the screen's y
    // points down); its radius and its line's thickness, in pixels.
    shape: vec4<f32>,
    // The share of the arc lit, from and to; how far the glow reaches, in pixels; how bright the
    // lit part burns, 1 being plain.
    fill: vec4<f32>,
    // The lit color, straight alpha.
    color: vec4<f32>,
    // The unlit line's color, straight alpha.
    dim: vec4<f32>,
    // How many pieces the arc is cut into (1 for a whole one); the gap between them, as a share of
    // the arc; where a notch crosses it, as a share (negative for none); how white the lit end
    // burns.
    cut: vec4<f32>,
};

@group(1) @binding(0) var<uniform> arc: Arc;

const PI: f32 = 3.14159265;

// Wraps an angle into [-π, π).
fn wrap(angle: f32) -> f32 {
    return angle - 2.0 * PI * floor((angle + PI) / (2.0 * PI));
}

// The point of the arc's circle at share `t` of the arc.
fn at(t: f32) -> vec2<f32> {
    let angle = arc.shape.x + arc.shape.y * t;
    return vec2<f32>(cos(angle), sin(angle)) * arc.shape.z;
}

// Distance from `p` to the piece of the arc from share `a` to share `b`, ends rounded, and the
// share of the arc `p` lies across from.
fn piece(p: vec2<f32>, a: f32, b: f32) -> vec2<f32> {
    let t = wrap(atan2(p.y, p.x) - arc.shape.x) / arc.shape.y;
    if (t >= a && t <= b) {
        return vec2<f32>(abs(length(p) - arc.shape.z), t);
    }
    let to_a = distance(p, at(a));
    let to_b = distance(p, at(b));
    if (to_a < to_b) {
        return vec2<f32>(to_a, a);
    }
    return vec2<f32>(to_b, b);
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = (in.uv - vec2<f32>(0.5)) * in.size;
    let half = arc.shape.w * 0.5;
    let pieces = max(arc.cut.x, 1.0);
    let gap = arc.cut.y;

    // The nearest piece of the arc.
    var best = vec2<f32>(1e9, 0.0);
    for (var i = 0.0; i < pieces; i = i + 1.0) {
        let a = (i + select(0.0, gap * 0.5, i > 0.0)) / pieces;
        let b = (i + 1.0 - select(0.0, gap * 0.5, i < pieces - 1.0)) / pieces;
        let found = piece(p, a, b);
        if (found.x < best.x) {
            best = found;
        }
    }
    let t = best.y;
    let lit = t >= arc.fill.x - 0.0001 && t <= arc.fill.y + 0.0001 && arc.fill.y > arc.fill.x;

    var rgb = vec3<f32>(0.0);
    var alpha = 0.0;
    if (lit) {
        // A crisp line, half a pixel of soft edge, brighter toward its lit end, in a tight glow
        // that falls off fast (since v1.0.0-rc7: thinner, the white core and the wide halo gone).
        let d = best.x - half;
        let inside = 1.0 - smoothstep(-0.5, 0.5, d);
        let along = clamp((t - arc.fill.x) / max(arc.fill.y - arc.fill.x, 0.0001), 0.0, 1.0);
        let shade = mix(0.6, 1.0, along) * arc.fill.w;
        // The lit end: a small spot of white, round, a little wider than the line.
        let head = exp(-distance(p, at(arc.fill.y)) / max(arc.shape.w * 0.9, 1.0)) * arc.cut.w;
        let line = mix(arc.color.rgb * shade, vec3<f32>(1.0), clamp(head, 0.0, 0.85));
        let out = max(d, 0.0) / max(arc.fill.z, 0.5);
        let glow = exp(-out * out * 2.0) * 0.4 * (1.0 - inside);
        rgb = line * inside + arc.color.rgb * shade * glow;
        alpha = max(inside, glow) * arc.color.a;
    } else {
        // Unlit: a hairline down the middle of where the line runs, quiet.
        let d = best.x - max(half * 0.35, 0.5);
        let inside = 1.0 - smoothstep(-0.5, 0.5, d);
        rgb = arc.dim.rgb;
        alpha = inside * arc.dim.a;
    }

    // The notch: a short white tick across the line.
    if (arc.cut.z >= 0.0) {
        let mark = at(arc.cut.z);
        let normal = normalize(mark);
        let tangent = vec2<f32>(-normal.y, normal.x);
        let along = abs(dot(p - mark, tangent));
        let out = abs(dot(p - mark, normal));
        let notch = (1.0 - smoothstep(0.5, 1.2, along)) * (1.0 - smoothstep(half + 2.5, half + 3.5, out));
        rgb = mix(rgb, vec3<f32>(1.0), notch);
        alpha = max(alpha, notch * 0.9);
    }
    return vec4<f32>(rgb, alpha);
}
