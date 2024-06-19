@group(0) @binding(0) var colorMap: texture_storage_2d<rgba8unorm, read_write>;
@group(0) @binding(1) var velocityMap: texture_storage_2d<rgba8unorm, read_write>;
@group(0) @binding(2) var divergenceMap: texture_storage_2d<rgba8unorm, read_write>;
@group(0) @binding(3) var pressureMap: texture_storage_2d<rgba8unorm, read_write>;

@compute @workgroup_size(8, 8, 1)
fn init(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    var color = vec4<f32>(0.0);
    if i32(invocation_id.x) < 100 && i32(invocation_id.y) < 300 {
        color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    }

    var velocity = vec4<f32>(0.0);
    if i32(invocation_id.x) < 100 && i32(invocation_id.y) < 200 {
        velocity = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    }
    textureStore(colorMap, location, color);
    textureStore(velocityMap, location, velocity);
    textureStore(divergenceMap, location, color);
    textureStore(pressureMap, location, color);
}

fn get_color(location: vec2<i32>, offset_x: i32, offset_y: i32) -> f32 {
    let value: vec4<f32> = textureLoad(colorMap, location + vec2<i32>(offset_x, offset_y));
    return value.x;
}

fn get_velocity(location: vec2<i32>, offset_x: i32, offset_y: i32) -> f32 {
    let value: vec4<f32> = textureLoad(velocityMap, location + vec2<i32>(offset_x, offset_y));
    return value.x;
}

fn get_divergence(location: vec2<i32>, offset_x: i32, offset_y: i32) -> f32 {
    let value: vec4<f32> = textureLoad(divergenceMap, location + vec2<i32>(offset_x, offset_y));
    return value.x;
}

fn get_pressure(location: vec2<i32>, offset_x: i32, offset_y: i32) -> f32 {
    let value: vec4<f32> = textureLoad(pressureMap, location + vec2<i32>(offset_x, offset_y));
    return value.x;
}

@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let velocity = get_velocity(location, 0, 0);
    let color = get_color(location, 0, 0);

    // if color is bigger than 0, decrease 0.01
    if velocity > 0.0 {
        textureStore(colorMap, location, vec4<f32>(color - 0.01));
    }
}
