struct Resolution{
   inputwidth:f32;
   inputheight:f32;
   outputwidth:f32;
   outputheight:f32;
   sharpnessrcas:f32,
   sharpnesslcas:f32,
};

let sharpStrength : f32 = 0.43;
let sharpClamp : f32 = 0.63;
let offsetBias : f32 = 1.0;

let CoefLuma : vec3<f32> = vec3<f32>(0.2126, 0.7152, 0.0722);



[[group(0),binding(0)]]
var ytexture: texture_2d<f32>;
[[group(0),binding(1)]]
var utexture: texture_2d<f32>;
[[group(0),binding(2)]]
var sam: sampler;
[[group(0),binding(3)]]
var<uniform> resolution:Resolution;

[[group(1),binding(0)]]
var store: texture_storage_2d<rgba8unorm, read_write>;

[[stage(compute) , workgroup_size(64,16,1)]]
fn cs_main([[builtin(global_invocation_id)]] invocation_id : vec3<u32>) {
    let location : vec2<f32> = vec2<f32>(f32(invocation_id.x),f32(invocation_id.y));
    var y = textureSampleLevel(ytexture,sam, location,0.0).r - 0.0625;
    var u = textureSampleLevel(utexture,sam, location,0.0).r - 0.5;
    var v = textureSampleLevel(utexture,sam, location,0.0).g - 0.5;
    
    
    //bt709
    // var r =  (y) + 1.793 * (v);
    // var g = (y) - 0.534 * (v) - 0.213 * (u);
    // var b = (y) + 2.115 * (u);
    
    //bt601
    var r = 1.164 * (y) + 1.596 * (v);
    var g = 1.164 * (y) - 0.813 * (v) - 0.392 * (u);
    var b = 1.164 * (y) + 2.017 * (u);
    
    var rgb : vec3<f32> = vec3<f32>(r,g,b);
    rgb = pow(rgb,vec3<f32>(2.2));
    let newrgb = vec4<f32>(rgb,1.0);
    textureStore(store,vec2<i32>(i32(invocation_id.x),i32(invocation_id.y)),newrgb);
}