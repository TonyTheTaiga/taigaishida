import datetime
from typing import List, Optional
from pathlib import Path
import os
import logging
from uuid import uuid4

from fastapi import FastAPI, HTTPException, Request, Depends, UploadFile, File, Form
from fastapi.responses import HTMLResponse
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates
from pydantic import BaseModel
import google.auth
from google.auth.transport import requests
from google.auth import compute_engine
from datetime import datetime, timedelta
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
        or blob.name.endswith(".jpg")
        or blob.name.endswith(".png")
        or blob.name.endswith(".jpeg")
        or blob.name.endswith(".WEBP")
        or blob.name.endswith(".JPG")
        or blob.name.endswith(".PNG")
        or blob.name.endswith(".JPEG")
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


class Item(BaseModel):
    filename: str
    passphrase: str
    content_type: str


@app.post("/get-upload-url")
def get_upload_url(item: Item):
    if item.passphrase != CORRECT_PASSPHRASE:
        raise HTTPException(status_code=403, detail="Incorrect passphrase")

    auth_request = requests.Request()
    signing_credentials = compute_engine.IDTokenCredentials(
        auth_request,
        "",
        service_account_email="storager@taigaishida-217622.iam.gserviceaccount.com",
    )

    # Generate a signed URL for uploading a file
    blob = client.bucket(IMAGE_BUCKET).blob(os.path.join(IMAGE_PREFIX, item.filename))
    url = blob.generate_signed_url(
        version="v4",
        expiration=timedelta(minutes=15),
        credentials=signing_credentials,
        method="PUT",
        content_type=item.content_type,
    )

    return {"uploadUrl": url}
