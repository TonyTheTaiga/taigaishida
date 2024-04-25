import base64

from openai import OpenAI
import cv2
import numpy as np

client = OpenAI()


def b64_encode_image(image: np.ndarray) -> bytes:
    _, im_arr = cv2.imencode(".jpg", image)
    im_bytes = im_arr.tobytes()
    return base64.b64encode(im_bytes)


def create_image_url(image):
    pass


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
                        "url": "https://upload.wikimedia.org/wikipedia/commons/thumb/d/dd/Gfp-wisconsin-madison-the-nature-boardwalk.jpg/2560px-Gfp-wisconsin-madison-the-nature-boardwalk.jpg",
                    },
                },
            ],
        }
    ],
    max_tokens=300,
)

print(response.choices[0])
