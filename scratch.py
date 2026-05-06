import re

with open('src/overlay.rs', 'r', encoding='utf-8') as f:
    text = f.read()

# 1. Update the capture offset logic to include virtual_matrix
# We want: if virtual_matrix is ON, the coordinates are ALREADY absolute (relative to virtual origin),
# and sx/sy should just be (coord * ppp). 
# Wait, wx is now either window_pos or virtual_origin.
# If virtual_matrix is ON, wx = virtual_origin.
# If we add wx to a relative coord, we get absolute screen coord.
# But if coord is ALREADY absolute (because win_offset was applied), we should NOT add wx.

# In the current code, many places do:
# let sx = (rect.min.x * ppp) as i32 + if settings.use_absolute_screen_coords { 0 } else { wx };

# If virtual_matrix is ON, we want:
# let sx = (rect.min.x * ppp) as i32 + if (settings.use_absolute_screen_coords || settings.virtual_matrix) { 0 } else { wx };

text = text.replace('if settings.use_absolute_screen_coords { 0 } else { wx }', 'if (settings.use_absolute_screen_coords || settings.virtual_matrix) { 0 } else { wx }')

# Also check for simple "+ wx" or "+ wy" that might need updating if they expect window-relative input
# e.g. at line 2248: let sx = (mouse.pos.x * ppp) as i32 + wx;
# Since wx is now virtual_origin if matrix is ON, this works IF mouse.pos.x is window-relative.

with open('src/overlay.rs', 'w', encoding='utf-8') as f:
    f.write(text)

print("Refined capture coordinates for Virtual Desktop Matrix")
