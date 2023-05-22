from typing import List, Optional
from pathlib import Path
import os
import logging
import io
from uuid import uuid4

from fastapi import FastAPI, Request, Depends, UploadFile, File
from fastapi.responses import HTMLResponse
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates
from pydantic import BaseModel
from google.cloud import storage
from PIL import Image

IMAGE_BUCKET = "taiga-ishida-public"
IMAGE_PREFIX = "webp_images"
GCS_API_ROOT = "https://storage.googleapis.com"

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
async def upload_image(image: UploadFile = File(...)):
    # Create a PIL Image object
    image_pil = Image.open(image.file)

    # Convert image to WebP format in memory
    byte_arr = io.BytesIO()
    image_pil.save(byte_arr, format="WEBP")
    byte_arr.seek(0)  # Move the cursor back to the beginning of the file

    # Generate a new filename
    filename = f"{IMAGE_PREFIX}/{uuid4()}.webp"

    blob = storage.Blob(filename, client.bucket(IMAGE_BUCKET))

    # Upload the image to GCS
    blob.upload_from_file(byte_arr, content_type="image/webp")

    return {"message": "Image uploaded successfully", "filename": filename}


@app.get("/upload", response_class=HTMLResponse)
async def upload(request: Request):
    return templates.TemplateResponse("upload.html", {"request": request})
