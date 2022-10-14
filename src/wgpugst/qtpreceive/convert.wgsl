struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

  

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}
fn lerp(a:f32,b:f32,percentage:f32)->f32{
      
	  return a + ((b - a) * percentage);
}
@group(0) @binding(0)
var ytexture: texture_2d<f32>;
@group(0) @binding(1)
var utexture: texture_2d<f32>;
// [[group(0),binding(2)]]
// var vtexture: texture_2d<f32>;
@group(0) @binding(2)
var sam: sampler;
let sharpness : f32 = 0.5;



@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    
	
    
    var y = textureSample(ytexture,sam, in.tex_coords).r - 0.0625;
    var u = textureSample(utexture,sam, in.tex_coords).r - 0.5;
    var v = textureSample(utexture,sam, in.tex_coords).g - 0.5;
    
    
    //bt709
    // var r =  (y) + 1.793 * (v);
    // var g = (y) - 0.534 * (v) - 0.213 * (u);
    // var b = (y) + 2.115 * (u);
    
    //bt601
    var r = 1.164 * (y) + 1.596 * (v);
    var g = 1.164 * (y) - 0.813 * (v) - 0.392 * (u);
    var b = 1.164 * (y) + 2.017 * (u);

    //bt601fr
    // var r = y + 1.402 * (v);
    // var g = y - 0.714 * (v) - 0.344 * (u);
    // var b = y + 1.772 * (u);

    //bt709fr
    // var r = y + 1.575 * (v);
    // var g = y - 0.468 * (v) - 0.187 * (u);
    // var b = y + 1.856 * (u);
    
    var rgb : vec3<f32> = vec3<f32>(r,g,b);
    rgb = pow(rgb,vec3<f32>(2.2));
    //For image that already have srgb
    //rgb = pow(rgb,vec3<f32>(1.0/2.2));
    return vec4<f32>(rgb,1.0) ;
}