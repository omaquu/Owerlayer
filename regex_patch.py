import re

with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\types.rs', 'r', encoding='utf-8') as f:
    t_content = f.read()

# PlacedImage from_image
t_content = re.sub(r'glow_strength: self\.glow_strength,\s*blur: self\.blur,', r'glow_strength: self.glow_strength,\n            glow_color: self.glow_color,\n            glow_spread: self.glow_spread,\n            blur: self.blur,', t_content)

# PlacedImage default
t_content = re.sub(r'glow: false,\s*glow_strength: 0\.0,\s*mask: None,', r'glow: false,\n            glow_strength: 0.0,\n            glow_color: [255, 255, 255, 255],\n            glow_spread: 0.0,\n            mask: None,', t_content)

with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\types.rs', 'w', encoding='utf-8') as f:
    f.write(t_content)

with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\project.rs', 'r', encoding='utf-8') as f:
    p_content = f.read()

p_content = re.sub(r'shadow_offset: default_shadow_offset\(\),', r'shadow_spread: 0.0,\n            shadow_offset: default_shadow_offset(),', p_content)

with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\project.rs', 'w', encoding='utf-8') as f:
    f.write(p_content)

with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\tools\text.rs', 'r', encoding='utf-8') as f:
    txt_content = f.read()

txt_content = re.sub(r'let c = egui::Color32::from_rgba_unmultiplied\(', r'let mut c = egui::Color32::from_rgba_unmultiplied(', txt_content)

with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\tools\text.rs', 'w', encoding='utf-8') as f:
    f.write(txt_content)

print("Regex patch applied.")
