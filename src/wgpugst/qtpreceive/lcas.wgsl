

let sharpness : f32 = 0.05;
struct Resolution{
   inputwidth:f32,
   inputheight:f32,
   outputwidth:f32,
   outputheight:f32,
   sharpnessrcas:f32,
   sharpnesslcas:f32,
}

@group(0)@binding(0)
var input: texture_2d<f32>;
@group(0)@binding(1)
var sam: sampler;

@group(1)@binding(0)// 1.
var<uniform> resolution:Resolution;

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
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>{
	let peak : f32 = lerp(0.0, -0.1111111111111111, resolution.sharpnesslcas);
	// let pos : vec2<f32>  = (ip + 0.5) * in.tex_coords;
    // var inputSize : vec2<f32>;
    let inputWidthRcas : f32 = resolution.outputwidth;
	let inputHeightRcas : f32 = resolution.outputheight;
    // inputSize.y = inputHeightRcas;
	// var outputSize : vec2<f32>;
    // outputSize.x = inputWidthRcas;
    // outputSize.y = inputHeightRcas;
	
    let pos : vec2<f32> = floor(in.tex_coords / vec2<f32>(inputWidthRcas, inputHeightRcas));
	let onemorex : f32 = 1.0 / inputWidthRcas;
	let onemorey : f32 = 1.0 / inputHeightRcas;

    // fetch a 3x3 neighborhood around the pixel 'e',
	//	a b c
	//	d(e)f
	//	g h i
	

	let a : vec3<f32> = textureSample(input,sam,in.tex_coords + vec2<f32>(-inputWidthRcas, -inputHeightRcas)).rgb;
	let b : vec3<f32> = textureSample(input,sam,in.tex_coords + vec2<f32>(0.0, -inputHeightRcas)).rgb;
	let c : vec3<f32> = textureSample(input,sam,in.tex_coords + vec2<f32>(inputWidthRcas, -inputHeightRcas)).rgb;
	let d : vec3<f32> = textureSample(input,sam,in.tex_coords + vec2<f32>(-inputWidthRcas, 0.0)).rgb;
	var e : vec3<f32> = textureSample(input,sam,in.tex_coords).rgb;
	let f : vec3<f32> = textureSample(input,sam,in.tex_coords + vec2<f32>(inputWidthRcas, 0.0)).rgb;
	let g : vec3<f32> = textureSample(input,sam,in.tex_coords + vec2<f32>(-inputWidthRcas, inputHeightRcas)).rgb;
	let h : vec3<f32> = textureSample(input,sam,in.tex_coords + vec2<f32>(0.0, inputHeightRcas)).rgb;
	let i : vec3<f32> = textureSample(input,sam,in.tex_coords + vec2<f32>(inputWidthRcas, inputHeightRcas)).rgb;

	let x : vec3<f32> = a + c + g + i;
	let y : vec3<f32> = b + d + f + h;

	//var e : vec3<f32> = textureSample(input,sam,vec2<f32>(in.tex_coords.x * 0.25, in.tex_coords.y * 0.5)).rgb;
    e = e + textureSample(input,sam,in.tex_coords + vec2<f32>(-inputWidthRcas * 0.25, -inputHeightRcas * 0.5)).rgb;
	e = e + textureSample(input,sam,in.tex_coords + vec2<f32>(inputWidthRcas * 0.5, -inputHeightRcas * 0.25)).rgb;
	e = e + textureSample(input,sam,in.tex_coords + vec2<f32>(-inputWidthRcas * 0.5, inputHeightRcas * 0.25)).rgb;
	e = e / 1.5;

    // Soft min and max.
	//  a b c
	//  d e f
	//  g h i
	let mnRGB : vec3<f32> = min(min(min(min(d, e), min(f, b)), h), min(min(a, i), min(c, g)));
	let mxRGB : vec3<f32> = max(max(max(max(d, e), max(f, b)), h), max(max(a, i), max(c, g)));

    // Shaping amount of sharpening.
	let wRGB : vec3<f32> = sqrt(min(mnRGB, 1.0 - mxRGB) / mxRGB) * peak;

	// Filter shape.
	//  w w w 
	//  w 1 w
	//  w w w 
	let color : vec3<f32> = ((x + y) * wRGB + (e * 5.0 - (x + y * 2.0 + e * 4.0) / 4.0)) / (1.0 + 8.0 * wRGB);
	return vec4<f32>((color + clamp(color, mnRGB, mxRGB) * 4.0) / 5.0,1.0);
}