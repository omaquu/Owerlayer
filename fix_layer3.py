with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\project.rs', 'r', encoding='utf-8') as f:
    c = f.read()

old = '''            shadow_offset: default_shadow_offset(),
            shadow_color: default_shadow_color(),'''
new = '''            shadow_offset: default_shadow_offset(),
            shadow_blur: 0.0,
            shadow_color: default_shadow_color(),'''

if old in c:
    c = c.replace(old, new)
    with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\project.rs', 'w', encoding='utf-8', newline='\n') as f:
        f.write(c)
    print('project.rs updated successfully.')
else:
    print('Pattern not found in project.rs')
