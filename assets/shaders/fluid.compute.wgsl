@group(0) @binding(0) var colorMap: texture_storage_2d<rgba8unorm, read_write>;
@group(0) @binding(1) var velocityXMap: texture_storage_2d<r32float, read_write>;
@group(0) @binding(2) var velocityYMap: texture_storage_2d<r32float, read_write>;
@group(0) @binding(3) var divergenceMap: texture_storage_2d<rgba8unorm, read_write>;
@group(0) @binding(4) var pressureMap: texture_storage_2d<rgba8unorm, read_write>;

@compute @workgroup_size(8, 8, 1)
fn init(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    var color = vec4<f32>(0.0);
    if i32(invocation_id.x) < 100 && i32(invocation_id.y) < 300 {
        color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    }

    var velocity_x = vec4<f32>(0.0);
    if i32(invocation_id.y) < 200 {
        velocity_x = vec4<f32>(2.0);
    }
    textureStore(colorMap, location, color);
    textureStore(velocityXMap, location, velocity_x);
    textureStore(velocityYMap, location, vec4<f32>(0.0));
}

fn get_color(location: vec2<i32>, offset: vec2<i32>) -> vec4<f32> {
    let value: vec4<f32> = textureLoad(colorMap, location + vec2<i32>(offset.x, offset.y));
    return value;
}

fn get_velocity(location: vec2<i32>, offset: vec2<i32>) -> vec2<f32> {
    let valueX = textureLoad(velocityXMap, location + vec2<i32>(offset.x, offset.y)).x;
    let valueY = textureLoad(velocityYMap, location + vec2<i32>(offset.x, offset.y)).x;
    return vec2<f32>(valueX, valueY);
}

fn get_divergence(location: vec2<i32>, offset_x: i32, offset_y: i32) -> f32 {
    let value: vec4<f32> = textureLoad(divergenceMap, location + vec2<i32>(offset_x, offset_y));
    return value.x;
}

fn get_pressure(location: vec2<i32>, offset_x: i32, offset_y: i32) -> f32 {
    let value: vec4<f32> = textureLoad(pressureMap, location + vec2<i32>(offset_x, offset_y));
    return value.x;
}

fn sample_velocity(location: vec2<i32>, offset: vec2<f32>) -> vec2<f32> {
    // オフセットの整数部分と小数部分を取得
    let offset_floor = vec2<i32>(i32(offset.x + 0.5), i32(offset.y + 0.5));
    let offset_fract = offset - vec2<f32>(offset_floor);

    // 周囲4つのセルの座標を計算
    let pos00 = location + offset_floor;

    // 周囲4つのセルの値を取得
    let value00 = get_velocity(pos00, vec2<i32>(0, 0));
    let value10 = get_velocity(pos00, vec2<i32>(1, 0));
    let value01 = get_velocity(pos00, vec2<i32>(0, 1));
    let value11 = get_velocity(pos00, vec2<i32>(1, 1));

    // 水平方向の補間
    let value0 = mix(value00, value10, offset_fract.x);
    let value1 = mix(value01, value11, offset_fract.x);

    // 垂直方向の補間
    let result = mix(value0, value1, offset_fract.y);

    return result;
}

fn sample_color(location: vec2<i32>, offset: vec2<f32>) -> vec4<f32> {
    // オフセットの整数部分と小数部分を取得
    let offset_floor = vec2<i32>(i32(offset.x + 0.5), i32(offset.y + 0.5));
    let offset_fract = offset - vec2<f32>(offset_floor);

    // 周囲4つのセルの座標を計算
    let pos00 = location + offset_floor;
    let pos10 = pos00 + vec2<i32>(1, 0);
    let pos01 = pos00 + vec2<i32>(0, 1);
    let pos11 = pos00 + vec2<i32>(1, 1);

    // 周囲4つのセルの値を取得
    let value00 = textureLoad(colorMap, pos00);
    let value10 = textureLoad(colorMap, pos10);
    let value01 = textureLoad(colorMap, pos01);
    let value11 = textureLoad(colorMap, pos11);

    // 水平方向の補間
    let value0 = mix(value00, value10, offset_fract.x);
    let value1 = mix(value01, value11, offset_fract.x);

    // 垂直方向の補間
    let result = mix(value0, value1, offset_fract.y);

    return result;
}

fn update_color(location: vec2<i32>) {
    // let velocity: vec2<f32> = get_velocity(location, vec2(0));
    // let velocity: vec2<f32> = -vec2(2.0, 2.0);
    let velocity = -get_velocity(location, vec2(0));
    let newColor = sample_color(location, velocity);
    // let newColor = sample_color(location, vec2(-2.0, -2.0));
    textureStore(colorMap, location, newColor);
}

// function updateColorMap() {
//   const newColorMap = [];
//   for (let y = 0; y < mapResolution; y++) {
//     newColorMap.push([]);
//     for (let x = 0; x < mapResolution; x++) {
//       const sx = x - velocityMap[y][x][0];
//       const sy = y - velocityMap[y][x][1];
// 			newColorMap[y].push(sampleMap(colorMap, sx, sy));
//     } 
//   }
//   colorMap = newColorMap;
// }

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
}
