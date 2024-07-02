@group(0) @binding(0) var colorMap: texture_storage_2d<rgba8unorm, read_write>;
@group(0) @binding(1) var growthMap: texture_storage_2d<r32float, read_write>;

const RESOLUTION = vec2<i32>(600, 400);
const ring_radius = 15;
const mu = 0.14;     // growth center
const sigma = 0.014; // growth width
const rho = 0.5;     // kernel center
const omega = 0.15;  // kernel width

fn wrap_coord(coord: vec2<i32>) -> vec2<i32> {
    let wrapped_x = (coord.x % RESOLUTION.x + RESOLUTION.x) % RESOLUTION.x;
    let wrapped_y = (coord.y % RESOLUTION.y + RESOLUTION.y) % RESOLUTION.y;
    return vec2<i32>(wrapped_x, wrapped_y);
}


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
    set_color(location, vec2<i32>(0, 0), color.x);
}

fn get_color(location: vec2<i32>, offset: vec2<i32>) -> f32 {
    let value: vec4<f32> = textureLoad(colorMap, wrap_coord(location + offset));
    return value.x;
}

fn get_growth(location: vec2<i32>, offset: vec2<i32>) -> f32 {
    let value: vec4<f32> = textureLoad(colorMap, wrap_coord(location + offset));
    return value.x;
}

fn set_color(location: vec2<i32>, offset: vec2<i32>, value: f32) {
    textureStore(colorMap, wrap_coord(location + offset), vec4<f32>(value, 0.0, 0.0, 1.0));
}

fn set_growth(location: vec2<i32>, offset: vec2<i32>, value: f32) {
    textureStore(growthMap, wrap_coord(location + offset), vec4<f32>(value, 0.0, 0.0, 1.0));
}

@compute @workgroup_size(8, 8, 1)
fn compute_growth(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    var current_status = get_color(location, vec2<i32>(0, 0));
    var sum: f32 = 0.0;
    var total: f32 = 0.0;

    for (var i = -ring_radius; i <= ring_radius; i++) {
        for (var j = -ring_radius; j <= ring_radius; j++) {
            let cell_val = get_color(location, vec2<i32>(i, j));
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

    set_growth(location, vec2<i32>(0, 0), g);
}

fn compute_flow(location: vec2<i32>) -> vec2<f32> {
    // use sobel filter to compute gradient
    var flow = vec2<f32>(0.0);

    flow.x = get_growth(location, vec2<i32>(-1, -1))
    + 2.0 * get_growth(location, vec2<i32>(-1, 0)) 
    + get_growth(location, vec2<i32>(-1, 1))
    - get_growth(location, vec2<i32>(1, -1))
    - 2.0 * get_growth(location, vec2<i32>(1, 0))
    - get_growth(location, vec2<i32>(1, 1));

    flow.y = get_growth(location, vec2<i32>(-1, -1))
    + 2.0 * get_growth(location, vec2<i32>(0, -1)) 
    + get_growth(location, vec2<i32>(1, -1))
    - get_growth(location, vec2<i32>(-1, 1))
    - 2.0 * get_growth(location, vec2<i32>(0, 1))
    - get_growth(location, vec2<i32>(1, 1));

    flow = normalize(flow);

    return flow;
}

@compute @workgroup_size(8, 8, 1)
fn apply_flow(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    // let flow = compute_flow(location);
    let flow = vec2<f32>(0.5, 0.0);
    let value = get_color(location, vec2<i32>(0, 0));

    // let alpha = saturate((value * 0.5) * (value * 0.5));
    let alpha = 0.0;

    // オフセットの整数部分と小数部分を取得
    let flow_floor = vec2<i32>(floor(flow));
    let flow_fract = flow - vec2<f32>(flow_floor);

    // 周囲4つのセルの座標を計算
    let pos00 = location + flow_floor;

    // 周囲4つのセルの値を取得
    let value00 = get_color(pos00, vec2<i32>(0, 0));
    let value10 = get_color(pos00, vec2<i32>(1, 0));
    let value01 = get_color(pos00, vec2<i32>(0, 1));
    let value11 = get_color(pos00, vec2<i32>(1, 1));

    // 各セルに分配する比率を計算
    let weight00 = (1.0 - flow_fract.x) * (1.0 - flow_fract.y);
    let weight10 = flow_fract.x * (1.0 - flow_fract.y);
    let weight01 = (1.0 - flow_fract.x) * flow_fract.y;
    let weight11 = flow_fract.x * flow_fract.y;

    let flow00 = value00 + weight00 * (1.0 - alpha);
    let flow10 = value10 + weight10 * (1.0 - alpha);
    let flow01 = value01 + weight01 * (1.0 - alpha);
    let flow11 = value11 + weight11 * (1.0 - alpha);

    set_color(pos00, vec2<i32>(0, 0), flow00);
    set_color(pos00, vec2<i32>(1, 0), flow10);
    set_color(pos00, vec2<i32>(0, 1), flow01);
    set_color(pos00, vec2<i32>(1, 1), flow11);
    set_color(location, vec2<i32>(0, 0), value * alpha);
}

