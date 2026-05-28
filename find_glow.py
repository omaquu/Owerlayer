with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\ui\object_fx.rs', 'r', encoding='utf-8') as f:
    c = f.read()

idx = c.find('ui.checkbox(&mut .glow, "Glow");')
if idx != -1:
    print(c[idx-50:idx+250])
