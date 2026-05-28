with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\gl_renderer.rs', 'r', encoding='utf-8') as f:
    c = f.read()

old_code = '''                    } else if (u_effect == 2) { // Pixelate
                        float pixel_size = max(1.0, u_strength);
                        vec2 p = uv * u_resolution;
                        p = floor(p / pixel_size) * pixel_size;
                        color = texture(u_sampler, p / u_resolution);
                    } else if (u_effect == 3) { // VHS Glitch
                        float strength = u_strength * 0.02;
                        float jitter = (rand(vec2(u_time, uv.y)) - 0.5) * strength;
                        vec2 jittered_uv = uv + vec2(jitter, 0.0);
                        
                        float r = texture(u_sampler, jittered_uv + vec2(strength * 0.5, 0.0)).r;
                        float g = texture(u_sampler, jittered_uv).g;
                        float b = texture(u_sampler, jittered_uv - vec2(strength * 0.5, 0.0)).b;
                        
                        float scanline = sin(uv.y * u_resolution.y * 0.8) * 0.05;
                        color = vec4(r - scanline, g - scanline, b - scanline, texture(u_sampler, jittered_uv).a);
                    }'''

new_code = '''                    } else if (u_effect == 2) { // Pixelate
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
                    }'''

if old_code in c:
    c = c.replace(old_code, new_code)
    with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\gl_renderer.rs', 'w', encoding='utf-8', newline='\n') as f:
        f.write(c)
    print('gl_renderer.rs patched.')
else:
    print('Pattern not found in gl_renderer.rs')
