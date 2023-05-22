from typing import List, Optional
from pathlib import Path
import os
import logging

from fastapi import FastAPI, Request, Depends
from fastapi.responses import HTMLResponse
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates
from pydantic import BaseModel
from google.cloud import storage


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
client = storage.Client.create_anonymous_client()


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
