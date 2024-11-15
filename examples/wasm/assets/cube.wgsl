struct Block {
    shape: array<array<i32, 4>, 4>, // 4x4 方块形状（可以根据需要调整大小）
    position: vec2<i32>,             // 当前方块的位置
    color: i32,                // 方块颜色
};

// 示例方块形状

const I_BLOCK_SHAPE: array<array<i32, 4>, 4> = array<array<i32, 4>, 4>(
    array<i32, 4>(0, 0, 0, 0),
    array<i32, 4>(1, 1, 1, 1),
    array<i32, 4>(0, 0, 0, 0),
    array<i32, 4>(0, 0, 0, 0)
);
const I_BLOCK_SHAPE1: array<array<i32, 4>, 4> = array<array<i32, 4>, 4>(
    array<i32, 4>(0, 1, 0, 0),
    array<i32, 4>(0, 1, 1, 1),
    array<i32, 4>(0, 0, 0, 0),
    array<i32, 4>(0, 0, 0, 0)
);
const I_BLOCK_SHAPE2: array<array<i32, 4>, 4> = array<array<i32, 4>, 4>(
    array<i32, 4>(0, 1, 0, 0),
    array<i32, 4>(0, 1, 1, 0),
    array<i32, 4>(0, 0, 1, 0),
    array<i32, 4>(0, 0, 0, 0)
);
const I_BLOCK_SHAPE3: array<array<i32, 4>, 4> = array<array<i32, 4>, 4>(
    array<i32, 4>(0, 0, 0, 0),
    array<i32, 4>(0, 1, 1, 0),
    array<i32, 4>(0, 1, 1, 0),
    array<i32, 4>(0, 0, 0, 0)
);
const I_BLOCK_SHAPE4: array<array<i32, 4>, 4> = array<array<i32, 4>, 4>(
    array<i32, 4>(0, 0, 0, 0),
    array<i32, 4>(0, 0, 1, 0),
    array<i32, 4>(0, 1, 1, 1),
    array<i32, 4>(0, 0, 0, 0)
);
var<private> BLOCKS:array<array<array<i32, 4>, 4>,5> = array(I_BLOCK_SHAPE, I_BLOCK_SHAPE1, I_BLOCK_SHAPE2, I_BLOCK_SHAPE3, I_BLOCK_SHAPE4);
var<private> Colors:array<vec4<f32>,7> = array(vec4(0, 0, 0, 1), vec4(1), vec4(1, 0, 0, 1), vec4(0, 0, 1, 1), vec4(0, 1, 0, 1), vec4(1, 1, 0, 1), vec4(1, 0, 1, 1));
const I_BLOCK: Block = Block(
    I_BLOCK_SHAPE,
    vec2<i32>(20, 0),
    1
);
struct SimParams {
    /// Delta time in seconds since last simulation tick.
    delta_time: f32,
    /// Time in seconds since the start of simulation.
    time: f32,
    /// Virtual delta time in seconds since last simulation tick.
    virtual_delta_time: f32,
    /// Virtual time in seconds since the start of simulation.
    virtual_time: f32,
    /// Real delta time in seconds since last simulation tick.
    real_delta_time: f32,
    /// Real time in seconds since the start of simulation.
    real_time: f32,
    /// Number of groups batched together.
    num_groups: u32,
    ping: u32,
}
struct Timer {
    now_time: f32,
    max_time: f32,
}

fn hash(x: f32) -> f32 {
    let p = fract(x * (misc.my_seed + .1031));
    let p_squared = p * p;
    misc.my_seed = fract(p * (p_squared + 33.33));
    return misc.my_seed;
}

// var <private> cube_now:Block = Block(
//     I_BLOCK_SHAPE,
//      vec2<i32>(20, 5),
//      vec3<f32>(0.0, 1.0, 1.0)
//      ); //假设中心在底部第1方块,左下角

struct Misc {
    is_touch: i32,
    dir: i32,
    my_seed: f32,
}

const alive_color = vec4<f32>(1.0);

@group(0) @binding(0) var<uniform> sim :SimParams;
@group(0) @binding(1) var<storage,read_write> stack: array<vec2<u32>,20000>;
@group(0) @binding(2) var<storage,read_write> visiable_map: array<i32,20000>;
@group(1) @binding(0) var<storage,read_write> pic :array<array<i32,100>,200>;
@group(1) @binding(1) var<storage,read_write> timer:Timer;
@group(1) @binding(2) var<storage,read_write> misc:Misc;
@group(1) @binding(3) var<storage,read_write> cube_now:Block;
@group(2) @binding(0) var<storage,read_write> index:u32;
@group(2) @binding(1) var<storage,read_write> index_two:u32;

@group(3) @binding(0) var output_texture: texture_storage_2d<rgba32float,write>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {

    let x = (global_invocation_id.x);
    if global_invocation_id.x == 0 {
        index_two = index + 1;
    }
    // if pic[index ][x] == 0 {
    //     textureStore(output_texture, vec2(x, index), vec4<f32>(0, 0, 0, 1));
    //     return;
    // }

    if global_invocation_id.x >= 100 {
        return;
    }
    // if(timer.now_time<timer.max_time){
    //     return;
    // }

    // if index <= 0 {
    //     textureStore(output_texture, vec2(x, index), Colors[ pic[index][x] ]);
    //     return;
    // }

    var i = index;
    //storageBarrier();
    //for (var i: u32 = 0; i < 200; i = i + 1) {
    if pic[i ][x] == 0 {
        textureStore(output_texture, vec2(x, i), vec4<f32>(0, 0, 0, 1));
        return;
    }
    if i <= 0 {
        textureStore(output_texture, vec2(x, i), Colors[ pic[i][x] ]);
        return;
    }
    let color = Colors[ pic[i][x] ];
    let last_color = vec4<f32>(0, 0, 0, 1);
    var pos = vec2(x, i);
    var last_pos = pos;
    var is_return = false;
    if !is_return && pic[i - 1][x] == 0 {
        pic[i - 1][x] = pic[i][x];
        pic[i][x] = 0;

        pos = vec2(x, i - 1) ;
        is_return = true;
    }
        //storageBarrier();
    if !is_return && (x) >= 0 + 1 && pic[i - 1][x - 1] == 0 {
        pic[i - 1][x - 1] = pic[i][x];
        pic[i][x] = 0;

        pos = vec2(x - 1, i - 1);
        is_return = true;
    }
        //storageBarrier();
    if !is_return && (x) < 100 - 1 && pic[i - 1][x + 1] == 0 {
        pic[i - 1][x + 1] = pic[i][x];
        pic[i][x] = 0;
        pos = vec2(x + 1, i - 1) ;
        is_return = true;
    }
    textureStore(output_texture, last_pos, last_color);
    textureStore(output_texture, pos, color);//Colors[ pic[index][x] ]
    //}
    //storageBarrier();
}


@compute @workgroup_size(4,4)
fn main_cube(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {

    var pos1 = vec2(global_invocation_id.x, global_invocation_id.y);
    if cube_now.shape[pos1.x][pos1.y] == 0 {
        return;
    }
    let color = Colors[cube_now.color];
    let pos = (cube_now.position) + vec2<i32>(pos1 * 10) - vec2(1, 0);
    if pos.y + misc.dir < 0 || pos.y + misc.dir + 10 > 100 {
        misc.dir = 0;
    }
    for (var i = pos.x; i < pos.x + 10; i = i + 1) {
        if i >= 200 || i <= 0 {
            if i < 0 {
                misc.is_touch = 1;
            }
            continue;
        }
        for (var j = pos.y; j < pos.y + 10; j = j + 1) {
            if j >= 100 || j < 0 {
                continue;
            }
            textureStore(output_texture, vec2(j, i), color);
            if pic[i][j] != 0 || i < 0 {
                misc.is_touch = 1;
            }
        }
    }
}

@compute @workgroup_size(4,4)
fn push_in(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    if 1 != misc.is_touch {
        return;
    }
    var pos1 = vec2(global_invocation_id.x, global_invocation_id.y);
    if cube_now.shape[pos1.x][pos1.y] == 0 {
        return;
    }
    var pos = vec2<u32>(cube_now.position) + pos1 * 10;


    for (var i = pos.x; i < pos.x + 10; i = i + 1) {
        if i >= 200 {
            break;
        }
        for (var j = pos.y; j < pos.y + 10; j = j + 1) {
            if j >= 100 {
                break;
            }
            pic[i][j] = cube_now.color;
        }
    }
}
@compute @workgroup_size(1)
fn check_and_spawn(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {

    for (var i :u32;i<20000;i=i+1) {
        visiable_map[i] = 0;
    }

    for (var i: u32 = 0; i < 200; i = i + 1) {
        var stack_index: i32 = 0;
        var stack_length: i32 = 0;
        var is_true: bool = false;
        if 1 == visiable_map[i * 100] {
            continue;
        } else {
            visiable_map[i * 100] = 1;
        }
        if pic[i][0] == 0 {
            continue;
        }
        stack[0] = vec2(i, 0);
        let now_target: i32 = pic[i][0];
        while stack_index <= stack_length {
            let x = stack[stack_index].x;
            let y = stack[stack_index].y;
            stack_index += 1;
            if x + 1 < 200 && 1 != visiable_map[(x + 1) * 100 + y] && pic[x + 1][y] == now_target {
                stack_length += 1;
                stack[stack_length] = vec2(x + 1, y);
                visiable_map[(x + 1) * 100 + y] = 1;
            }
            if y + 1 < 100 && 1 != visiable_map[(x) * 100 + y + 1] && pic[x][y + 1] == now_target {
                stack_length += 1;
                stack[stack_length] = vec2(x, y + 1);
                if y + 1 == 99 {
                    is_true = true;
                }
                visiable_map[(x) * 100 + y + 1] = 1;
            }
            if x > 0 && 1 != visiable_map[(x - 1) * 100 + y] && pic[x - 1][y] == now_target {
                stack_length += 1;
                stack[stack_length] = vec2(x - 1, y);
                visiable_map[(x - 1) * 100 + y] = 1;
            }
            if y > 0 && 1 != visiable_map[(x) * 100 + y - 1] && pic[x][y - 1] == now_target {
                stack_length += 1;
                stack[stack_length] = vec2(x, y - 1);
                visiable_map[(x) * 100 + y - 1] = 1;
            }
        }
        if is_true {
            for (var j: i32 = 0; j <= stack_length; j = j + 1) {
                pic[stack[j].x][stack[j].y] = 0;
            }
        }
    }

    if 1 != misc.is_touch {
        if global_invocation_id.x == 0 && global_invocation_id.y == 0 {
            cube_now.position = cube_now.position + vec2<i32>(-1, misc.dir);
        }
        return;
    }
    let index = u32(hash(sim.time) * 5);
    cube_now = Block(
        BLOCKS[index],
        vec2<i32>(200, 30),
        (i32(hash(sim.time) * 6) + 1),
    );
}
