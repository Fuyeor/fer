# python3 generate_lavender_logo.py
from PIL import Image, ImageDraw

def generate_perfect_logo(output_path="icon.png", target_size=(800, 800)):
    # create a new image with a transparent background
    img = Image.new("RGBA", target_size, (255, 255, 255, 0))
    draw = ImageDraw.Draw(img)
    
    width, height = target_size

    # # 15% of the width and height
    # margin_x = int(width * 0.15)
    # margin_y = int(height * 0.15)

    # # calculate the coordinates of the rounded rectangle
    # x0 = margin_x
    # y0 = margin_y
    # x1 = width - margin_x
    # y1 = height - margin_y

    draw.rounded_rectangle(
        # [x0, y0, x1, y1],
        [0, 0, width, height], 
        radius=150, 
        fill="#AEA4E4"
    )
    
    # save the image as a PNG file
    img.save(output_path, "PNG")

generate_perfect_logo()
