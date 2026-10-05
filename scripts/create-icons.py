"""Render the geometric XTools mark into the native Windows icon sizes."""
from pathlib import Path
from PIL import Image, ImageDraw

root = Path(__file__).resolve().parents[1] / "src-tauri" / "icons"
root.mkdir(parents=True, exist_ok=True)
im = Image.new("RGBA", (1024, 1024))
d = ImageDraw.Draw(im)
d.rounded_rectangle((32, 32, 992, 992), radius=232, fill="#222a35")
for polygon in [[(65,67),(108,67),(191,189),(148,189)],[(145,67),(191,67),(157,117),(113,117)],[(100,139),(145,139),(111,189),(65,189)]]:
    d.polygon([(x*4,y*4) for x,y in polygon], fill="#f7f8fa")
im = im.resize((256,256), Image.Resampling.LANCZOS)
im.save(root / "icon.png")
im.save(root / "icon.ico", sizes=[(16,16),(24,24),(32,32),(48,48),(64,64),(128,128),(256,256)])
