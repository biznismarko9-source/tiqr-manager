"""Generates the base 1024x1024 TIQR Manager app icon (a simple, professional
ticket glyph on a brand-color rounded square) used as input to `tauri icon`,
which then derives every platform-specific icon size/format from it.
"""
from PIL import Image, ImageDraw, ImageFont

SIZE = 1024
BG = (52, 72, 235, 255)  # brand-600
BG2 = (74, 104, 247, 255)  # brand-500 (subtle gradient feel via two bands)
WHITE = (255, 255, 255, 255)

img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
draw = ImageDraw.Draw(img)

# Rounded square background
radius = 200
draw.rounded_rectangle([0, 0, SIZE - 1, SIZE - 1], radius=radius, fill=BG)
# Subtle top-light band for depth
draw.rounded_rectangle([0, 0, SIZE - 1, SIZE // 2], radius=radius, fill=BG2)
draw.rectangle([0, SIZE // 2 - radius, SIZE - 1, SIZE // 2], fill=BG2)
draw.rounded_rectangle([0, 0, SIZE - 1, SIZE - 1], radius=radius, outline=None)

# Re-clip to rounded corners by masking
mask = Image.new("L", (SIZE, SIZE), 0)
mdraw = ImageDraw.Draw(mask)
mdraw.rounded_rectangle([0, 0, SIZE - 1, SIZE - 1], radius=radius, fill=255)
img.putalpha(mask)

draw = ImageDraw.Draw(img)

# Ticket glyph: rounded rect with notches cut on left/right edges + a dashed
# perforation line, rendered in white over the brand background.
tw, th = 620, 360
tx, ty = (SIZE - tw) // 2, (SIZE - th) // 2 + 30
ticket_mask = Image.new("L", (SIZE, SIZE), 0)
tdraw = ImageDraw.Draw(ticket_mask)
tdraw.rounded_rectangle([tx, ty, tx + tw, ty + th], radius=36, fill=255)
notch_r = 46
for cx in (tx, tx + tw):
    tdraw.ellipse([cx - notch_r, ty + th // 2 - notch_r, cx + notch_r, ty + th // 2 + notch_r], fill=0)
ticket_img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
ticket_img.paste(WHITE, (0, 0, SIZE, SIZE), ticket_mask)
img.alpha_composite(ticket_img)

# Perforation dashed line (brand color dashes cut through the white ticket)
perf_x = tx + int(tw * 0.62)
dash_h, gap, y = 26, 20, ty + 24
while y < ty + th - 24:
    draw.rectangle([perf_x - 6, y, perf_x + 6, y + dash_h], fill=BG)
    y += dash_h + gap

# Big "T" monogram on the ticket stub area
try:
    font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 190)
except Exception:
    font = ImageFont.load_default()
label = "T"
bbox = draw.textbbox((0, 0), label, font=font)
lw, lh = bbox[2] - bbox[0], bbox[3] - bbox[1]
lx = tx + int(tw * 0.30) - lw // 2 - bbox[0]
ly = ty + th // 2 - lh // 2 - bbox[1]
draw.text((lx, ly), label, font=font, fill=BG)

img.save("/home/claude/tiqr-manager/src-tauri/icons/app-icon-source.png")
print("wrote icon source")
