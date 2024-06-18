@group(0) @binding(0) var texture: texture_storage_2d<rgba8unorm, read_write>;

const ring_radius = 15;
const mu = 0.14;     // growth center
const sigma = 0.014; // growth width
const rho = 0.5;     // kernel center
const omega = 0.15;  // kernel width

fn hash(p: vec2<f32>) -> f32 {
    let p2 = dot(p, vec2<f32>(127.1, 311.7));
    return -1.0 + 2.0 * fract(sin(p2) * 43758.5453123);
}

fn bell(x: f32, mu: f32, sigma: f32) -> f32 {
    // bell curve
    return exp(-((x - mu) * (x - mu)) / (2.0 * sigma * sigma));
}
fn growth(U: f32, m: f32, s: f32) -> f32 {
    let g = bell(U, m, s) * 2.0 - 1.0;
    return g;
}

@compute @workgroup_size(8, 8, 1)
fn init(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    // let randomNumber = randomFloat(invocation_id.y * num_workgroups.x + invocation_id.x);
    let randomNumber = hash(vec2<f32>(invocation_id.xy));
    var color = vec4<f32>(0.0);
    if i32(invocation_id.x) < 100 && i32(invocation_id.y) < 300 {
        color = vec4<f32>(randomNumber);
    }
    textureStore(texture, location, color);
}

fn get_value(location: vec2<i32>, offset_x: i32, offset_y: i32) -> f32 {
    let value: vec4<f32> = textureLoad(texture, location + vec2<i32>(offset_x, offset_y));
    return value.x;
}

fn compute_new_state(location: vec2<i32>) -> f32 {
    var current_status = get_value(location, 0, 0);
    var sum: f32 = 0.0;
    var total: f32 = 0.0;

    for (var i = -ring_radius; i <= ring_radius; i++) {
        for (var j = -ring_radius; j <= ring_radius; j++) {
            let cell_val = get_value(location, i, j);
            let i_f = f32(i);
            let j_f = f32(j);
            let r = sqrt((i_f * i_f) + (j_f * j_f)) / f32(ring_radius);
            let weight = bell(r, rho, omega);
            sum += cell_val * weight;
            total += weight;
        }
    }

    let avg = sum / total;
    let g = bell(avg, mu, sigma) * 2.0 - 1.0;
    // change kernel depending on current_status
    // let g = growth(avg * (1.0 + (current_status - 0.5)*0.2), mu, sigma);
    let result = saturate(current_status + 0.1 * g);
    return result;
}

@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let new_state = compute_new_state(location);
    let new_value = vec4<f32>(new_state);

    storageBarrier();
    let color = vec4<f32>(new_value);
    textureStore(texture, location, color);
}

// @fragment
// fn fs_main() -> @location(0) vec4<f32> {
//     // let color = textureLoad(texture, i32(gl_FragCoord.xy));
//     return vec4<f32>(color.rgb, 1.0);
// }
