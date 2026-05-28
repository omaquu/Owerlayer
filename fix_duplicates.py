import re

with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\types.rs', 'r', encoding='utf-8') as f:
    t_content = f.read()

# Fix duplicates in PlacedImage clone
t_content = re.sub(
    r'shadow_color: self\.shadow_color,\s*shadow_offset: self\.shadow_offset,\s*shadow_spread: self\.shadow_spread,\s*shadow_blur: self\.shadow_blur,\s*glow: self\.glow,\s*glow_strength: self\.glow_strength,\s*shadow_color: self\.shadow_color,\s*shadow_offset: self\.shadow_offset,\s*shadow_spread: self\.shadow_spread,\s*shadow_blur: self\.shadow_blur,\s*glow: self\.glow,\s*glow_strength: self\.glow_strength,',
    r'shadow_color: self.shadow_color,\n            shadow_offset: self.shadow_offset,\n            shadow_spread: self.shadow_spread,\n            shadow_blur: self.shadow_blur,\n            glow: self.glow,\n            glow_strength: self.glow_strength,\n            glow_color: self.glow_color,\n            glow_spread: self.glow_spread,',
    t_content
)

# And if there are single duplicates
t_content = re.sub(r'(shadow_color: self\.shadow_color,[\s\S]*?)shadow_color: self\.shadow_color,', r'\1', t_content, count=1)
t_content = re.sub(r'(shadow_offset: self\.shadow_offset,[\s\S]*?)shadow_offset: self\.shadow_offset,', r'\1', t_content, count=1)
t_content = re.sub(r'(shadow_spread: self\.shadow_spread,[\s\S]*?)shadow_spread: self\.shadow_spread,', r'\1', t_content, count=1)
t_content = re.sub(r'(shadow_blur: self\.shadow_blur,[\s\S]*?)shadow_blur: self\.shadow_blur,', r'\1', t_content, count=1)
t_content = re.sub(r'(glow: self\.glow,[\s\S]*?)glow: self\.glow,', r'\1', t_content, count=1)
t_content = re.sub(r'(glow_strength: self\.glow_strength,[\s\S]*?)glow_strength: self\.glow_strength,', r'\1', t_content, count=1)

# Fix missing glow_color and glow_spread in the bottom of PlacedImage constructor
t_content = re.sub(
    r'glow: false,\s*glow_strength: 0\.0,\s*rotation: 0\.0,',
    r'glow: false,\n            glow_strength: 0.0,\n            glow_color: [255, 255, 255, 255],\n            glow_spread: 0.0,\n            rotation: 0.0,',
    t_content
)

with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\types.rs', 'w', encoding='utf-8') as f:
    f.write(t_content)

print("Duplicates and missing fields fixed.")
