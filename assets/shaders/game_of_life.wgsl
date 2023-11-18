@group(0) @binding(0) var texture: texture_storage_2d<rgba8unorm, read_write>;

const ring_radius = 2;

fn hash(value: u32) -> u32 {
    var state = value;
    state = state ^ 2747636419u;
    state = state * 2654435769u;
    state = state ^ state >> 16u;
    state = state * 2654435769u;
    state = state ^ state >> 16u;
    state = state * 2654435769u;
    return state;
}

fn randomFloat(value: u32) -> f32 {
    return f32(hash(value)) / 4294967295.0;
}

@compute @workgroup_size(8, 8, 1)
fn init(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let randomNumber = randomFloat(invocation_id.y * num_workgroups.x + invocation_id.x);
    let color = vec4<f32>(randomNumber);
    textureStore(texture, location, color);
}

fn get_value(location: vec2<i32>, offset_x: i32, offset_y: i32) -> f32 {
    let value: vec4<f32> = textureLoad(texture, location + vec2<i32>(offset_x, offset_y));
    return value.x;
}

fn compute_weighted_sum(location: vec2<i32>) -> f32 {
    var sum: f32 = 0.0;

    for (var i = -ring_radius; i <= ring_radius; i = i + 1) {
        for (var j = -ring_radius; j <= ring_radius; j = j + 1) {
            // weight according to the euclidean distance from center
            let distance = (i * i) + (j * j);
            // let weight = 1.0 - f32(distance) / 3.0;
            let weight = 1.0;
            sum = sum + weight * get_value(location, i, j);
        }
    }
    return sum;
}

@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let weighted_sum = compute_weighted_sum(location);
    let new_value = weighted_sum / 25.0;  // normalize the weighted sum

    storageBarrier();

    let color = vec4<f32>(new_value);
    textureStore(texture, location, color);
}
