with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\gl_renderer.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Fix Bug 2 - use sample_tex in default branch to prevent CLAMP_TO_EDGE line artifact
old = '''                    } else {
                        color = texture(u_sampler, uv);
                    }'''
new = '''                    } else {
                        color = sample_tex(u_sampler, uv);
                    }'''

if old in content:
    content = content.replace(old, new)
    with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\gl_renderer.rs', 'w', encoding='utf-8', newline='\n') as f:
        f.write(content)
    print('gl_renderer.rs patched - sample_tex fix applied')
else:
    print('ERROR: target not found')
