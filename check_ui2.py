with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\ui\object_fx.rs', 'r', encoding='utf-8') as f:
    content = f.read()
idx = content.find('Enable Drop Shadow')
print(repr(content[idx:idx+800]))
