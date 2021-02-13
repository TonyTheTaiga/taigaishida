import os
from PIL import Image

TARGET_DIR = 'images'
OUTPUT_DIR = 'compressed_images'


def main():
    for image in os.listdir(TARGET_DIR):
        if image == '.DS_Store':
            continue

        im = Image.open(os.path.join(TARGET_DIR,image))
        
        if not os.path.exists(os.path.join(OUTPUT_DIR, '{}'.format(image)).lower()):
            try:
                im.save(os.path.join(OUTPUT_DIR, '{}'.format(image)).lower(), format='JPEG', quality=70)
            except:
                im.save(os.path.join(OUTPUT_DIR, '{}'.format(image)).lower(), format='PNG', quality=70)


if __name__ == '__main__':
    main()
