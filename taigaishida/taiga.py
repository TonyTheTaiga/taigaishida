from typing import List, Optional, Annotated
from pathlib import Path
import os
import logging
import io
from uuid import uuid4

from fastapi import FastAPI, HTTPException, Request, Depends, UploadFile, File, Form
from fastapi.responses import HTMLResponse
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates
from pydantic import BaseModel
from google.cloud import storage
import numpy as np
import cv2

IMAGE_BUCKET = "taiga-ishida-public"
IMAGE_PREFIX = "webp_images"
GCS_API_ROOT = "https://storage.googleapis.com"
CORRECT_PASSPHRASE = "greengrass123"

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("ui")

app = FastAPI()

app.mount(
    "/static",
    StaticFiles(directory=Path(__file__).parent.resolve() / "static"),
    name="static",
)
templates = Jinja2Templates(directory=Path(__file__).parent.resolve() / "templates")

client = storage.Client()


def convert_bytes_to_image(image_bytes) -> np.ndarray:
    return cv2.imdecode(np.frombuffer(image_bytes, dtype=np.uint8), cv2.IMREAD_COLOR)


def convert_image_to_bytes(image, extension, params=None):
    ret, buf = cv2.imencode(extension, image, params)
    if not ret:
        raise RuntimeError("Failed to convert image to bytes")

    return buf.tobytes()


class GalleryItem(BaseModel):
    id: str
    name: Optional[str] = "???"

    @property
    def url(self) -> str:
        return os.path.join(GCS_API_ROOT, IMAGE_BUCKET, self.id)


def get_gallery_items() -> List[GalleryItem]:
    blobs = client.list_blobs(IMAGE_BUCKET, prefix=IMAGE_PREFIX)
    return [
        GalleryItem(id=blob.name, name=blob.name.split("/")[-1])
        for blob in blobs
        if blob.name.endswith(".webp")
    ]


@app.get("/gallery", response_class=HTMLResponse)
@app.get("/", response_class=HTMLResponse)
async def gallery(request: Request, gallery_items=Depends(get_gallery_items)):
    return templates.TemplateResponse(
        "gallery.html",
        {"request": request, "gallery": [item.url for item in gallery_items]},
    )


@app.get("/about", response_class=HTMLResponse)
async def about(request: Request):
    return templates.TemplateResponse("about.html", {"request": request})


@app.post("/upload-image")
def upload_image(image: UploadFile = File(...), passphrase: str = Form(...)):
    if passphrase != CORRECT_PASSPHRASE:
        raise HTTPException(status_code=403, detail="Incorrect passphrase")

    image = convert_bytes_to_image(image.file.read())
    webp_image_bytes = convert_image_to_bytes(
        image, ".webp", params=[cv2.IMWRITE_WEBP_QUALITY, 75]
    )

    filename = f"{IMAGE_PREFIX}/{uuid4()}.webp"
    blob = storage.Blob(filename, client.bucket(IMAGE_BUCKET))

    # Upload the image to GCS
    blob.upload_from_string(webp_image_bytes, content_type="image/webp")

    return {"message": "Image uploaded successfully", "filename": filename}


@app.get("/upload", response_class=HTMLResponse)
async def upload(request: Request):
    return templates.TemplateResponse("upload.html", {"request": request})
