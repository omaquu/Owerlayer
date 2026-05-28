import re

with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\types.rs', 'r', encoding='utf-8') as f:
    t_content = f.read()

# Fix missing fields for PlacedImage::from_url
t_content = re.sub(
    r'glow: false,\s*glow_strength: 0\.0,\s*rotation: 0\.0,\s*flipped_h: false, flipped_v: false,',
    r'glow: false,\n            glow_strength: 0.0,\n            glow_color: [255, 255, 255, 255],\n            glow_spread: 0.0,\n            rotation: 0.0,\n            flipped_h: false, flipped_v: false,',
    t_content
)

with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\types.rs', 'w', encoding='utf-8') as f:
    f.write(t_content)

print("Final missing fields fixed.")
