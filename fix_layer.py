with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\project.rs', 'r', encoding='utf-8') as f:
    c = f.read()

old = '''      #[serde(default = "default_shadow_offset")]
      pub shadow_offset: [f32; 2],'''
new = '''      #[serde(default = "default_shadow_offset")]
      pub shadow_offset: [f32; 2],
      #[serde(default)]
      pub shadow_blur: f32,'''

c = c.replace(old, new)
with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\project.rs', 'w', encoding='utf-8', newline='\n') as f:
    f.write(c)
