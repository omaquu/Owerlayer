with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\ui\object_fx.rs', 'r', encoding='utf-8') as f:
    c = f.read()

import re

# find the macro body
macro_start = c.find('macro_rules! render_object_fx {')
macro_end = c.find('                match sel.object_type {')

if macro_start != -1 and macro_end != -1:
    body = c[macro_start:macro_end]
    inner_start = body.find('{\n                        section_heading')
    inner_end = body.rfind('};\n                }')
    if inner_start != -1 and inner_end != -1:
        inner = body[inner_start+1:inner_end]
        
        inner = inner.replace('', 'layer').replace('', 'false')
        
        with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\ui\filter_menu.rs', 'r', encoding='utf-8') as fm:
            fc = fm.read()
            
        fc_start = fc.find('                let layer = &mut project.layers[idx];\n')
        fc_end = fc.find('                if ui.button("Close").clicked() {')
        
        if fc_start != -1 and fc_end != -1:
            new_fc = fc[:fc_start + len('                let layer = &mut project.layers[idx];\n')] + inner + '\n                ui.add_space(8.0);\n\n                ' + fc[fc_end:]
            with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\ui\filter_menu.rs', 'w', encoding='utf-8', newline='\n') as fm_out:
                fm_out.write(new_fc)
            print("filter_menu.rs synced successfully!")
        else:
            print("Failed to find replacement block in filter_menu.rs")
    else:
        print("Failed to find inner block of macro")
else:
    print("Failed to find macro in object_fx.rs")
