with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\overlay.rs', 'r', encoding='utf-8') as f:
    c = f.read()

# The current block:
glow_block = '''                draw_pass(false, true, 0.0, 0.0, 0.0, 0.0, [1.0, 1.0, 1.0, 1.0], l_op * img.opacity);

                let has_glow = layer.glow || img.glow;
                if has_glow {
                    let (g_col_arr, g_str, g_spread) = if img.glow { 
                        (img.glow_color, img.glow_strength, img.glow_spread) 
                    } else {
                        (layer.glow_color, layer.glow_strength, layer.glow_spread)
                    };
                    let alpha = (g_str / 100.0).clamp(0.0, 1.0);
                    let tint = [g_col_arr[0] as f32 / 255.0, g_col_arr[1] as f32 / 255.0, g_col_arr[2] as f32 / 255.0, g_col_arr[3] as f32 / 255.0 * alpha];
                    draw_pass(true, false, 0.0, 0.0, g_spread, 10.0, tint, l_op * img.opacity);
                }'''

new_glow_block = '''                let has_glow = layer.glow || img.glow;
                if has_glow {
                    let (g_col_arr, g_str, g_spread) = if img.glow { 
                        (img.glow_color, img.glow_strength, img.glow_spread) 
                    } else {
                        (layer.glow_color, layer.glow_strength, layer.glow_spread)
                    };
                    let alpha = (g_str / 100.0).clamp(0.0, 1.0);
                    let tint = [g_col_arr[0] as f32 / 255.0, g_col_arr[1] as f32 / 255.0, g_col_arr[2] as f32 / 255.0, g_col_arr[3] as f32 / 255.0 * alpha];
                    // Render glow silhouette
                    draw_pass(true, false, 0.0, 0.0, g_spread, 10.0, tint, l_op * img.opacity);
                }

                draw_pass(false, true, 0.0, 0.0, 0.0, 0.0, [1.0, 1.0, 1.0, 1.0], l_op * img.opacity);'''

if glow_block in c:
    c = c.replace(glow_block, new_glow_block)
    with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\overlay.rs', 'w', encoding='utf-8', newline='\n') as f:
        f.write(c)
    print("overlay.rs patched.")
else:
    print("Could not find glow_block in overlay.rs")
