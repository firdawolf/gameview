
@group(0) @binding(0)
var ytexture: texture_2d<f32>;
@group(0) @binding(1)
var uvtexture: texture_2d<f32>;
@group(0) @binding(2)
var samp: sampler;
@group(0) @binding(3) 
var rgbstorage : texture_storage_2d<rgba8unorm, write>;


// var<workgroup> ytile : array<array<f32, 128>, 4>;
// var<workgroup> uvtile : array<array<vec2<f32>, 128>, 4>;

@compute @workgroup_size(32)
fn main(@builtin(workgroup_id) WorkGroupID : vec3<u32>,
  @builtin(global_invocation_id) global_id: vec3<u32>,
  @builtin(local_invocation_id) LocalInvocationID : vec3<u32>) {
    let ydims : vec2<i32> = textureDimensions(ytexture, 0);
    let uvdims : vec2<i32> = textureDimensions(uvtexture, 0);
    let baseIndex : vec2<i32> = vec2<i32>(
    global_id.xy
    );
    
      
      
      let y:f32 = textureSampleLevel(
        ytexture,
        samp,
        vec2<f32>(baseIndex) / vec2<f32>(ydims), 
        0.0
      ).r - 0.0625;
      let u:f32 = textureSampleLevel(
        uvtexture,
        samp,
        vec2<f32>(baseIndex) / vec2<f32>(ydims), 
        0.0 
      ).r - 0.5;
      let v:f32 = textureSampleLevel(
        uvtexture,
        samp,
        vec2<f32>(baseIndex) / vec2<f32>(ydims), 
        0.0
      ).g - 0.5;
      var r = 1.164 * (y) + 1.596 * (v);
      var g = 1.164 * (y) - 0.813 * (v) - 0.392 * (u);
      var b = 1.164 * (y) + 2.017 * (u);
      var rgb : vec3<f32> = vec3<f32>(r,g,b);
      rgb = pow(rgb,vec3<f32>(2.2));
      textureStore(rgbstorage, baseIndex, vec4<f32>(rgb, 1.0));
   
  //   for (var r1 : u32 = 0u; r1 < 4u; r1 = r1 + 1u) {
  //   for (var c1 : u32 = 0u; c1 < 4u; c1 = c1 + 1u) {
  //     var loadIndex = baseIndex + vec2<i32>(i32(c1), i32(r1));
      
  //     let y:f32 = textureSampleLevel(
  //       ytexture,
  //       samp,
  //       vec2<f32>(loadIndex) / vec2<f32>(ydims), 
  //       0.0
  //     ).r - 0.0625;
  //     let u:f32 = textureSampleLevel(
  //       uvtexture,
  //       samp,
  //       vec2<f32>(loadIndex) / vec2<f32>(uvdims), 
  //       0.0 
  //     ).r - 0.5;
  //     let v:f32 = textureSampleLevel(
  //       uvtexture,
  //       samp,
  //       vec2<f32>(loadIndex) / vec2<f32>(uvdims), 
  //       0.0
  //     ).g - 0.5;
  //     var r = 1.164 * (y) + 1.596 * (v);
  //     var g = 1.164 * (y) - 0.813 * (v) - 0.392 * (u);
  //     var b = 1.164 * (y) + 2.017 * (u);
  //     var rgb : vec3<f32> = vec3<f32>(r,g,b);
  //     rgb = pow(rgb,vec3<f32>(2.2));
  //     textureStore(rgbstorage, loadIndex, vec4<f32>(rgb, 1.0));
  //   }
  // }
//   workgroupBarrier();
//   for (var r1 : u32 = 0u; r1 < 4u; r1 = r1 + 1u) {
//     for (var c : u32 = 0u; c < 4u; c = c + 1u) {
//       var writeIndex : vec2<i32> = baseIndex + vec2<i32>(i32(c), i32(r1));
//       var r = 1.164 * (ytile[r1][4u * LocalInvocationID.x + c]) + 1.596 * (uvtile[r1][4u * LocalInvocationID.x + c].y);
//       var g = 1.164 * (ytile[r1][4u * LocalInvocationID.x + c]) - 0.813 * (uvtile[r1][4u * LocalInvocationID.x + c].y) - 0.392 * (uvtile[r1][4u * LocalInvocationID.x + c].x);
//       var b = 1.164 * (ytile[r1][4u * LocalInvocationID.x + c]) + 2.017 * (uvtile[r1][4u * LocalInvocationID.x + c].x);
       
//       var rgb : vec3<f32> = vec3<f32>(r,g,b);
//       rgb = pow(rgb,vec3<f32>(2.2));
//       textureStore(rgbstorage, writeIndex, vec4<f32>(rgb, 1.0));
//     }
//   }
    
    
    
}