use eframe::glow::{self, HasContext};
use std::sync::Arc;

pub struct GLRenderer {
    pub program: glow::Program,
    pub vertex_array: glow::VertexArray,
    pub vertex_buffer: glow::Buffer,
    // Caches
    pub u_sampler: Option<glow::UniformLocation>,
    pub u_mask: Option<glow::UniformLocation>,
    pub u_has_mask: Option<glow::UniformLocation>,
    pub u_effect: Option<glow::UniformLocation>,
    pub u_strength: Option<glow::UniformLocation>,
    pub u_resolution: Option<glow::UniformLocation>,
    pub u_time: Option<glow::UniformLocation>,
    pub u_gray: Option<glow::UniformLocation>,
    pub u_inv: Option<glow::UniformLocation>,
    pub u_sep: Option<glow::UniformLocation>,
    pub u_tint: Option<glow::UniformLocation>,
    pub u_shadow_mode: Option<glow::UniformLocation>,
    pub u_opacity: Option<glow::UniformLocation>,
    pub u_srgb_enabled: Option<glow::UniformLocation>,
}

impl GLRenderer {
    pub fn new(gl: &Arc<glow::Context>) -> Self {
        unsafe {
            let program = gl.create_program().expect("Cannot create program");

            let vertex_shader_source = r#"
                #version 330 core
                layout (location = 0) in vec2 a_pos;
                layout (location = 1) in vec2 a_uv;
                out vec2 v_uv;
                void main() {
                    v_uv = a_uv;
                    gl_Position = vec4(a_pos, 0.0, 1.0);
                }
            "#;

            let fragment_shader_source = r#"
                #version 330 core
                precision mediump float;
                uniform sampler2D u_sampler;
                uniform sampler2D u_mask;
                uniform int u_has_mask;
                uniform int u_effect; // 0: None, 1: Blur, 2: Pixelate, 3: VHS, 4: Grayscale, 5: Invert, 6: Sepia
                uniform float u_strength;
                uniform vec2 u_resolution;
                uniform float u_time;
                
                // Extra filter toggles
                uniform int u_grayscale;
                uniform int u_invert;
                uniform int u_sepia;
                uniform vec4 u_tint;
                uniform int u_shadow_mode;
                uniform float u_opacity;
                uniform int u_srgb_enabled;

                in vec2 v_uv;
                out vec4 f_color;

                float rand(vec2 co) {
                    return fract(sin(dot(co.xy ,vec2(12.9898,78.233))) * 43758.5453);
                }

                vec4 sample_tex(sampler2D samp, vec2 coord) {
                    vec2 clamped = clamp(coord, vec2(0.0), vec2(1.0));
                    return texture(samp, clamped);
                }

                void main() {
                    vec2 uv = v_uv;
                    vec4 color = vec4(0.0);

                    if (u_effect == 1) { // Blur (Box blur approximation)
                        int samples = 4;
                        vec2 tex_offset = 1.0 / u_resolution * (u_strength / 4.0);
                        for(int x = -samples; x <= samples; x++) {
                            for(int y = -samples; y <= samples; y++) {
                                color += sample_tex(u_sampler, uv + vec2(float(x), float(y)) * tex_offset);
                            }
                        }
                        color /= 81.0;
                    } else if (u_effect == 2) { // Pixelate
                        float pixel_size = max(1.0, u_strength);
                        vec2 p = uv * u_resolution;
                        p = floor(p / pixel_size) * pixel_size;
                        color = sample_tex(u_sampler, p / u_resolution);
                    } else if (u_effect == 3) { // VHS Glitch
                        float strength = u_strength * 0.02;
                        float jitter = (rand(vec2(u_time, uv.y)) - 0.5) * strength;
                        vec2 jittered_uv = uv + vec2(jitter, 0.0);
                        
                        float r = sample_tex(u_sampler, jittered_uv + vec2(strength * 0.5, 0.0)).r;
                        float g = sample_tex(u_sampler, jittered_uv).g;
                        float b = sample_tex(u_sampler, jittered_uv - vec2(strength * 0.5, 0.0)).b;
                        
                        float scanline = sin(uv.y * u_resolution.y * 0.8) * 0.05;
                        color = vec4(r - scanline, g - scanline, b - scanline, sample_tex(u_sampler, jittered_uv).a);
                    } else {
                        color = sample_tex(u_sampler, uv);
                    }

                    // Apply secondary filters
                    float a = color.a;
                    if (a > 0.0) {
                        color.rgb /= a;
                    }

                    if (u_grayscale != 0) {
                        float gray = dot(color.rgb, vec3(0.299, 0.587, 0.114));
                        color.rgb = vec3(gray);
                    }
                    if (u_invert != 0) {
                        color.rgb = 1.0 - color.rgb;
                    }
                    if (u_sepia != 0) {
                        color.rgb = vec3(
                            dot(color.rgb, vec3(0.393, 0.769, 0.189)),
                            dot(color.rgb, vec3(0.349, 0.686, 0.168)),
                            dot(color.rgb, vec3(0.272, 0.534, 0.131))
                        );
                    }

                    color.rgb *= a;

                    if (u_shadow_mode != 0) {
                        color.rgb = u_tint.rgb * color.a * u_tint.a;
                        color.a *= u_tint.a;
                    } else {
                        color.rgb *= u_tint.rgb;
                        color.a *= u_tint.a;
                    }

                    if (u_has_mask != 0) {
                        float m = texture(u_mask, v_uv).r;
                        color.rgb *= m;
                        color.a *= m;
                    }
                    color.rgb *= u_opacity;
                    color.a *= u_opacity;

                    if (u_srgb_enabled == 0) {
                        color.rgb = mix(
                            color.rgb * 12.92,
                            1.055 * pow(clamp(color.rgb, 0.0, 1.0), vec3(1.0 / 2.4)) - 0.055,
                            step(0.0031308, color.rgb)
                        );
                    }

                    f_color = color;
                }
            "#;

            let shader_sources = [
                (glow::VERTEX_SHADER, vertex_shader_source),
                (glow::FRAGMENT_SHADER, fragment_shader_source),
            ];

            let mut shaders = Vec::with_capacity(shader_sources.len());
            for (shader_type, shader_source) in shader_sources {
                let shader = gl.create_shader(shader_type).expect("Cannot create shader");
                gl.shader_source(shader, shader_source);
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    panic!("{}", gl.get_shader_info_log(shader));
                }
                gl.attach_shader(program, shader);
                shaders.push(shader);
            }

            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!("{}", gl.get_program_info_log(program));
            }

            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }

            let vertex_array = gl.create_vertex_array().expect("Cannot create VAO");
            gl.bind_vertex_array(Some(vertex_array));

            let vertex_buffer = gl.create_buffer().expect("Cannot create VBO");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));

            // Full screen quad
            let vertices = [
                -1.0, -1.0,  0.0, 1.0,
                 1.0, -1.0,  1.0, 1.0,
                -1.0,  1.0,  0.0, 0.0,
                 1.0,  1.0,  1.0, 0.0,
            ];
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytemuck::cast_slice(&vertices), glow::STATIC_DRAW);

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 4 * 4, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, 4 * 4, 2 * 4);

            let u_sampler = gl.get_uniform_location(program, "u_sampler");
            let u_mask = gl.get_uniform_location(program, "u_mask");
            let u_has_mask = gl.get_uniform_location(program, "u_has_mask");
            let u_effect = gl.get_uniform_location(program, "u_effect");
            let u_strength = gl.get_uniform_location(program, "u_strength");
            let u_resolution = gl.get_uniform_location(program, "u_resolution");
            let u_time = gl.get_uniform_location(program, "u_time");
            let u_gray = gl.get_uniform_location(program, "u_grayscale");
            let u_inv = gl.get_uniform_location(program, "u_invert");
            let u_sep = gl.get_uniform_location(program, "u_sepia");
            let u_tint = gl.get_uniform_location(program, "u_tint");
            let u_shadow_mode = gl.get_uniform_location(program, "u_shadow_mode");
            let u_opacity = gl.get_uniform_location(program, "u_opacity");
            let u_srgb_enabled = gl.get_uniform_location(program, "u_srgb_enabled");

            Self {
                program,
                vertex_array,
                vertex_buffer,
                u_sampler,
                u_mask,
                u_has_mask,
                u_effect,
                u_strength,
                u_resolution,
                u_time,
                u_gray,
                u_inv,
                u_sep,
                u_tint,
                u_shadow_mode,
                u_opacity,
                u_srgb_enabled,
            }
        }
    }

    pub fn create_texture(&self, gl: &glow::Context, width: u32, height: u32, pixels: &[u8]) -> glow::Texture {
        unsafe {
            let tex = gl.create_texture().expect("Cannot create texture");
            gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                width as i32,
                height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(pixels)),
            );
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
            tex
        }
    }

    pub fn render_effect(
        &self,
        gl: &glow::Context,
        texture: glow::Texture,
        mask_texture: Option<glow::Texture>,
        effect_type: i32,
        strength: f32,
        resolution: [f32; 2],
        time: f32,
        grayscale: bool,
        invert: bool,
        sepia: bool,
        tint: [f32; 4],
        shadow_mode: bool,
        opacity: f32,
        vertex_count: i32,
        vertices: &[f32],
    ) {
        unsafe {
            // Bind our own VAO and VBO to avoid corrupting egui's state
            gl.bind_vertex_array(Some(self.vertex_array));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vertex_buffer));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytemuck::cast_slice(vertices), glow::DYNAMIC_DRAW);

            gl.use_program(Some(self.program));

            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.uniform_1_i32(self.u_sampler.as_ref(), 0);

            if let Some(mask) = mask_texture {
                gl.active_texture(glow::TEXTURE1);
                gl.bind_texture(glow::TEXTURE_2D, Some(mask));
                gl.uniform_1_i32(self.u_mask.as_ref(), 1);
                gl.uniform_1_i32(self.u_has_mask.as_ref(), 1);
            } else {
                gl.uniform_1_i32(self.u_has_mask.as_ref(), 0);
            }

            gl.uniform_1_i32(self.u_effect.as_ref(), effect_type);
            gl.uniform_1_f32(self.u_strength.as_ref(), strength);
            gl.uniform_2_f32(self.u_resolution.as_ref(), resolution[0], resolution[1]);
            gl.uniform_1_f32(self.u_time.as_ref(), time);

            gl.uniform_1_i32(self.u_gray.as_ref(), grayscale as i32);
            gl.uniform_1_i32(self.u_inv.as_ref(), invert as i32);
            gl.uniform_1_i32(self.u_sep.as_ref(), sepia as i32);
            gl.uniform_4_f32(self.u_tint.as_ref(), tint[0], tint[1], tint[2], tint[3]);
            gl.uniform_1_i32(self.u_shadow_mode.as_ref(), shadow_mode as i32);
            gl.uniform_1_f32(self.u_opacity.as_ref(), opacity);

            let srgb_enabled = gl.is_enabled(glow::FRAMEBUFFER_SRGB);
            gl.uniform_1_i32(self.u_srgb_enabled.as_ref(), srgb_enabled as i32);

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 16, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, 16, 8);

            gl.draw_arrays(glow::TRIANGLES, 0, vertex_count);

            // Clean up to be absolutely safe
            gl.bind_vertex_array(None);
            gl.use_program(None);
        }
    }

    pub fn destroy(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.vertex_array);
            gl.delete_buffer(self.vertex_buffer);
        }
    }
}
