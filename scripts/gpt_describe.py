import base64

import cv2
import numpy as np
from openai import OpenAI

client = OpenAI()


def b64_encode_image(image: np.ndarray) -> bytes:
    _, im_arr = cv2.imencode(".jpg", image)
    im_bytes = im_arr.tobytes()
    return base64.b64encode(im_bytes)


def encode_image(image_path):
    with open(image_path, "rb") as image_file:
        return base64.b64encode(image_file.read()).decode("utf-8")


format_image = lambda base64_image: f"data:image/webp;base64,{base64_image}"
image = encode_image("/Users/taiga/Downloads/lifeguard.webp")
response = client.chat.completions.create(
    model="gpt-4-turbo",
    messages=[
        {
            "role": "user",
            "content": [
                {"type": "text", "text": "Create a haiku about this message."},
                {
                    "type": "image_url",
                    "image_url": {
                        # "url": "https://upload.wikimedia.org/wikipedia/commons/thumb/d/dd/Gfp-wisconsin-madison-the-nature-boardwalk.jpg/2560px-Gfp-wisconsin-madison-the-nature-boardwalk.jpg",
                        "url": format_image(image),
                    },
                },
            ],
        }
    ],
    max_tokens=300,
)

print(response.choices[0])
