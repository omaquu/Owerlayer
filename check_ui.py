with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\ui\object_fx.rs', 'r', encoding='utf-8') as f:
    content = f.read()
print('=== SHADOW SECTION ===')
idx = content.find('Enable Drop Shadow')
print(repr(content[idx:idx+400]))
print()
print('=== GLOW SECTION ===')
idx2 = content.find('checkbox(&mut .glow')
print(repr(content[idx2:idx2+300]))
