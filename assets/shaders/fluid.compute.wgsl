@group(0) @binding(0) var colorMap: texture_storage_2d<rgba8unorm, read_write>;
@group(0) @binding(1) var velocityXMap: texture_storage_2d<r32float, read_write>;
@group(0) @binding(2) var velocityYMap: texture_storage_2d<r32float, read_write>;
@group(0) @binding(3) var pressureMap: texture_storage_2d<r32float, read_write>;

const RED = vec4<f32>(1.0, 0.0, 0.0, 1.0);
const GREEN = vec4<f32>(0.0, 1.0, 0.0, 1.0);
const BLUE = vec4<f32>(0.0, 0.0, 1.0, 1.0);
const rho = 1.00;
// const rho = 0.99;
const RESOLUTION = vec2<i32>(600, 400);

const ring_radius = 25;
const mu = 0.14;     // growth center
const sigma = 0.014; // growth width
const rho2 = 0.5;     // kernel center
const omega = 0.15;  // kernel width


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
    var color = vec4<f32>(0.0);
    var velocity_x = vec4<f32>(0.0);
    var velocity_y = vec4<f32>(0.0);

    // make color pattern
    let colorArea = location.x % 300;
    if colorArea < 100 {
        color = RED;
    } else if colorArea < 200 {
        // color = GREEN;
    } else {
        // color = BLUE;
    }

    if i32(location.x) < 300 {
        velocity_y = vec4<f32>(-0.2);
    } else {
        velocity_y = vec4<f32>(0.2);
    }

    if i32(location.y) < 200 {
        velocity_x = vec4<f32>(0.3);
    } else {
        velocity_x = vec4<f32>(-0.3);
    }

    var pressure = vec4<f32>(0.0);
    // // if 250 < x < 350 && 150 < y < 250 {
    // if 250 < location.x && location.x < 350 && 150 < location.y && location.y < 250 {
    //     pressure = vec4<f32>(-50.0);
    // }
    textureStore(colorMap, location, color);
    textureStore(velocityXMap, location, velocity_x);
    textureStore(velocityYMap, location, velocity_y);
    textureStore(pressureMap, location, pressure);
}

fn wrap_coord(coord: vec2<i32>) -> vec2<i32> {
    let wrapped_x = (coord.x % RESOLUTION.x + RESOLUTION.x) % RESOLUTION.x;
    let wrapped_y = (coord.y % RESOLUTION.y + RESOLUTION.y) % RESOLUTION.y;
    return vec2<i32>(wrapped_x, wrapped_y);
}

fn get_color(location: vec2<i32>) -> vec4<f32> {
    let value: vec4<f32> = textureLoad(colorMap, wrap_coord(location));
    return value;
}

fn get_velocity(location: vec2<i32>) -> vec2<f32> {
    let valueX = textureLoad(velocityXMap, wrap_coord(location)).x;
    let valueY = textureLoad(velocityYMap, wrap_coord(location)).x;
    return vec2<f32>(valueX, valueY);
}

fn get_pressure(location: vec2<i32>) -> f32 {
    let value: vec4<f32> = textureLoad(pressureMap, wrap_coord(location));
    return value.x;
}

fn sample_velocity(pos: vec2<f32>) -> vec2<f32> {
    // オフセットの整数部分と小数部分を取得
    let pos00 = vec2<i32>(floor(pos));
    // let pos00 = vec2<i32>(floor(pos + vec2<f32>(0.5)));
    let pos_fract = fract(pos);

    // 周囲4つのセルの値を取得
    let value00 = get_velocity(pos00 + vec2<i32>(0, 0));
    let value10 = get_velocity(pos00 + vec2<i32>(1, 0));
    let value01 = get_velocity(pos00 + vec2<i32>(0, 1));
    let value11 = get_velocity(pos00 + vec2<i32>(1, 1));

    // 水平方向の補間
    let value0 = mix(value00, value10, pos_fract.x);
    let value1 = mix(value01, value11, pos_fract.x);

    // 垂直方向の補間
    let result = mix(value0, value1, pos_fract.y);

    return result;
}

fn update_color(location: vec2<i32>) {
    let velocity = get_velocity(location + vec2(0));

    let pos = vec2<f32>(location) - velocity;

        // オフセットの整数部分と小数部分を取得
    let pos00 = vec2<i32>(floor(pos));
    // let pos00 = vec2<i32>(floor(pos + vec2<f32>(0.5)));
    let pos_fract = fract(pos);

    // 周囲4つのセルの値を取得
    let value00 = get_color(pos00 + vec2<i32>(0, 0));
    let value10 = get_color(pos00 + vec2<i32>(1, 0));
    let value01 = get_color(pos00 + vec2<i32>(0, 1));
    let value11 = get_color(pos00 + vec2<i32>(1, 1));

    // 各セルに分配する比率を計算
    let weight00 = (1.0 - pos_fract.x) * (1.0 - pos_fract.y);
    let weight10 = pos_fract.x * (1.0 - pos_fract.y);
    let weight01 = (1.0 - pos_fract.x) * pos_fract.y;
    let weight11 = pos_fract.x * pos_fract.y;

    // 水平方向の補間
    let value0 = mix(value00, value10, pos_fract.x);
    let value1 = mix(value01, value11, pos_fract.x);

    // 垂直方向の補間
    let newColor = mix(value0, value1, pos_fract.y);


    set_color(pos00, vec2<i32>(0, 0), flow00);
    set_color(pos00, vec2<i32>(1, 0), flow10);
    set_color(pos00, vec2<i32>(0, 1), flow01);
    set_color(pos00, vec2<i32>(1, 1), flow11);
    set_color(location, vec2<i32>(0, 0), value * alpha);


    // textureStore(colorMap, location, newColor);
    textureStore(colorMap, wrap_coord(location), newColor);
}

fn update_velocity(location: vec2<i32>) {
    let velocity = -get_velocity(location + vec2(0));
    let newVelocity: vec2<f32> = sample_velocity(vec2<f32>(location) + velocity);
    // textureStore(velocityXMap, location, vec4(newVelocity.x));
    // textureStore(velocityYMap, location, vec4(newVelocity.y));
    textureStore(velocityXMap, wrap_coord(location), vec4(newVelocity.x));
    textureStore(velocityYMap, wrap_coord(location), vec4(newVelocity.y));
}

fn calc_divergence(location: vec2<i32>) -> f32 {
    let left_in = get_velocity(location + vec2(-1,0)).x;
    let right_in = get_velocity(location + vec2(1,0)).x;
    let top_in = get_velocity(location + vec2(0,-1)).y;
    let bottom_in = get_velocity(location + vec2(0,1)).y;
    let result = left_in - right_in + top_in - bottom_in;
    return result;
}

@compute @workgroup_size(8, 8, 1)
fn update_pressure(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let divergence = calc_divergence(location);

    let left_in = get_pressure(location + vec2(-1,0));
    let right_in = get_pressure(location + vec2(1,0));
    let top_in = get_pressure(location + vec2(0,-1));
    let bottom_in = get_pressure(location + vec2(0,1));

    let result = 0.25 * (divergence + left_in + right_in + top_in + bottom_in);
    // for (var i = 0; i < 1000; i++) {
    //     textureStore(pressureMap, wrap_coord(location), vec4(result));
    // }
    textureStore(pressureMap, wrap_coord(location), vec4(result));
}

fn gradient_subtract(location: vec2<i32>) {
    let left_pressure = get_pressure(location + vec2(-1,0));
    let right_pressure = get_pressure(location + vec2(1,0));
    let top_pressure = get_pressure(location + vec2(0,-1));
    let bottom_pressure = get_pressure(location + vec2(0,1));

    let pressure_diff_x = (right_pressure - left_pressure) * 0.5;
    let pressure_diff_y = (bottom_pressure - top_pressure) * 0.5;

    let velocity = get_velocity(location + vec2(0));

    let final_velocity_x = velocity.x - pressure_diff_x / rho;
    let final_velocity_y = velocity.y - pressure_diff_y / rho;

    // let final_velocity = normalize(vec2<f32>(final_velocity_x, final_velocity_y));
    let final_velocity = vec2<f32>(final_velocity_x, final_velocity_y);

    textureStore(velocityXMap, wrap_coord(location), vec4(final_velocity.x));
    textureStore(velocityYMap, wrap_coord(location), vec4(final_velocity.y));
}

@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    // let velocity = get_velocity(location, 0, 0);
    // let color = get_color(location, 0, 0);

    // if color is bigger than 0, decrease 0.01
    // if velocity > 0.0 {
    //     textureStore(colorMap, location, vec4<f32>(color - 0.01));
    // }
    update_color(location);
    update_velocity(location);
    
    // update_pressure(location, divergence);
    gradient_subtract(location);
    // if 250 < location.x && location.x < 350 && 150 < location.y && location.y < 250 {
    //     textureStore(pressureMap, location, vec4(1.0));
    // }
}
