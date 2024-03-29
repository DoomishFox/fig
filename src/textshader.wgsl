struct TextUniform {
    size: vec2<u32>,
    position: vec2<u32>,
    length: u32,
    kerning: u32,
};

struct CharData {
    atlas_offset_x: u32,
    atlas_offset_y: u32,
    color: vec3<f32>,
}

@group(0) @binding(0)
var<uniform> screen_size: vec2<u32>;

// visible to both vertex and fragment
// contains metadata like
// - character clip width x height
// - text starting position (clip coords)
// - array of characters
@group(0) @binding(1)
var<uniform> metadata: TextUniform;

@group(0) @binding(2)
var<storage, read> glyphs: array<CharData>;

// glyph atlas texture
@group(1) @binding(0)
var t_glyph: texture_2d<f32>;
@group(1) @binding(1)
var s_glyph: sampler;

fn pixel_to_world_coord_2d(
    pixel: vec2<u32>,
) -> vec2<f32> {
    return vec2<f32>(
        (f32(pixel[0]) / (f32(screen_size[0]) * 0.5)) - 1.0,
        (f32(pixel[1]) / (f32(screen_size[1]) * 0.5)) - 1.0
    );
}

fn pixel_to_world_dist_1d(
    pixel: u32,
) -> f32 {
    return f32(pixel) / (f32(screen_size[0]) * 0.5);
}

fn pixel_to_world_dist_2d(
    pixel: vec2<u32>,
) -> vec2<f32> {
    //return vec2<f32>(0.0, 0.0);
    return vec2<f32>(
        f32(pixel[0]) / (f32(screen_size[0]) * 0.5),
        f32(pixel[1]) / (f32(screen_size[1]) * 0.5)
    );
}

// ====== Vertex shader ======
struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) index: u32,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec3<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vert_index: u32,
    @builtin(instance_index) in_index: u32,
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = pixel_to_world_coord_2d(metadata.position);
    // move char based on index
    let char = model.position + vec3<f32>(f32(in_index) + (f32(in_index) * pixel_to_world_dist_1d(metadata.kerning)), 0.0, 0.0);
    //let char = model.position + vec3<f32>(f32(in_index) + (f32(in_index) * 20.0), 0.0, 0.0);
    //let char = model.position + vec3<f32>(f32(in_index), 0.0, 0.0);
    // scale char to proper aspect
    let aspect_char = char * vec3<f32>(5.0 / 12.0, 1.0, 1.0);
    // scale char based on metadata
    let scaled_char = aspect_char * vec3<f32>(pixel_to_world_dist_2d(metadata.size), 1.0);
    // position char in worldspace
    let positioned_char = scaled_char + vec3<f32>(world_pos, 0.0);
    out.clip_position = vec4<f32>(positioned_char, 1.0);
    out.index = in_index;
    // calculate texcoords here because instance_index isnt available
    // in the fragment shader because fuck you
    out.tex_coords = vec2<f32>(
        (model.position.x + f32(glyphs[in_index].atlas_offset_x)) / 32.0,
        ((1.0 - model.position.y) + f32(glyphs[in_index].atlas_offset_y)) / 12.0);
    out.color = glyphs[in_index].color;
    return out;
}

// ====== Fragment shader ======
@fragment
fn fs_main(
    in: VertexOutput
) -> @location(0) vec4<f32> {
    return (textureSample(t_glyph, s_glyph, in.tex_coords) * vec4<f32>(in.color, 1.0));
}
