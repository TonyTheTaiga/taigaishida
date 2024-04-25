import logging
import os
from datetime import timedelta
from pathlib import Path
from typing import List, Optional
from functools import lru_cache
from datetime import datetime
import json

import cv2
import google.auth
import numpy as np
from fastapi import Depends, FastAPI, HTTPException, Request
from fastapi.responses import HTMLResponse
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates
from google.auth import compute_engine
from google.auth.transport import requests
from google.cloud import storage, datastore
from google.cloud.datastore.query import PropertyFilter
from pydantic import BaseModel
import exifread

IMAGE_BUCKET = "taiga-ishida-public"
IMAGE_PREFIX = "webp_images"
GCS_API_ROOT = "https://storage.googleapis.com"
CORRECT_PASSPHRASE = "greengrass123"

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("ui")

IMAGE_KIND = "Image"

app = FastAPI()

app.mount(
    "/static",
    StaticFiles(directory=Path(__file__).parent.resolve() / "static"),
    name="static",
)
templates = Jinja2Templates(directory=Path(__file__).parent.resolve() / "templates")

client = storage.Client()


@lru_cache()
def get_client() -> storage.Client:
    logger.debug(f"creating client")
    return storage.Client()


@lru_cache()
def get_ds_client() -> datastore.Client:
    logger.debug(f"creating ds client")
    return datastore.Client()


def convert_bytes_to_image(image_bytes) -> np.ndarray:
    return cv2.imdecode(np.frombuffer(image_bytes, dtype=np.uint8), cv2.IMREAD_COLOR)


def convert_image_to_bytes(image, extension, params=None):
    ret, buf = cv2.imencode(extension, image, params)  # pyright: ignore
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


@app.get("/upload", response_class=HTMLResponse)
async def upload(request: Request):
    return templates.TemplateResponse("upload.html", {"request": request})


class Item(BaseModel):
    filename: str
    passphrase: str
    content_type: str


class RegisterImageRequest(BaseModel):
    filename: str


def dms_to_decimal(dms):
    """
    Convert a DMS (Degrees, Minutes, Seconds) tuple to decimal degrees.

    Args:
    dms (tuple): A tuple representing the DMS coordinates.
                 For example, (degrees, minutes, seconds_as_fraction).

    Returns:
    float: The decimal degree representation of the DMS input.
    """

    # Unpack the degrees, minutes, and seconds
    degrees, minutes, seconds_fraction = (
        dms.replace(",", "").lstrip("[").rstrip("]").split(" ")
    )
    # Convert the seconds fraction if it's a string fraction
    if isinstance(seconds_fraction, str) and "/" in seconds_fraction:
        numerator, denominator = map(float, seconds_fraction.split("/"))
        seconds_fraction = numerator / denominator

    # Convert to decimal degrees
    decimal_degrees = float(degrees) + (float(minutes) / 60) + (seconds_fraction / 3600)

    return decimal_degrees


def convert_datetime_to_iso_format(datetime_str):
    # Parse the original datetime string
    dt = datetime.strptime(datetime_str, "%Y:%m:%d %H:%M:%S")

    # Convert it to the ISO format with milliseconds and 'Z' (for UTC)
    formatted_datetime = dt.strftime("%Y-%m-%dT%H:%M:%S.000Z")

    return formatted_datetime


def read_exif(image_buf) -> dict:
    tags = exifread.process_file(image_buf, details=False)  # pyright: ignore

    # Might need to gracefully handle values that may not cast into strings nicely
    tags = {k.split(" ")[-1]: str(v).strip() for k, v in tags.items()}
    tags["created"] = convert_datetime_to_iso_format(tags["DateTime"])

    try:
        tags[
            "latlong"
        ] = f'{dms_to_decimal(tags["GPSLatitude"]):.5f} {tags["GPSLatitudeRef"]}, {dms_to_decimal(tags["GPSLongitude"]):.5f} {tags["GPSLongitudeRef"]}'
    except Exception as e:
        pass

    # Convert lat long to a city, Earth if unknown
    # tags['city'] = ...

    return tags


def create_entity(kind: str, data: dict, client=None) -> datastore.Entity:
    if client is None:
        client = datastore.Client()

    entity = datastore.Entity(client.key(kind))
    entity.update(data)
    return entity


@app.post("/register-image")
def register_image(request: RegisterImageRequest):
    client = get_client()
    ds_client = get_ds_client()
    blob = storage.Blob.from_string(
        f"gs://{IMAGE_BUCKET}/dev/staging/{request.filename}", client=client
    )
    assert blob.exists(), "blob doesn't exist"
    blob.reload()
    q = ds_client.query(kind=IMAGE_KIND)
    q.add_filter(filter=PropertyFilter("md5", "=", str(blob.md5_hash)))

    if not len(list(q.fetch())) == 0:
        blob.delete()
        raise HTTPException(
            status_code=409, detail="image with this md5 already exists"
        )

    tags = read_exif(blob.open(mode="rb"))
    data = {"name": request.filename, "md5": str(blob.md5_hash), **tags}
    entity = create_entity(IMAGE_KIND, data, ds_client)
    ds_client.put(entity)


@app.post("/get-upload-url")
def get_upload_url(item: Item):
    if item.passphrase != CORRECT_PASSPHRASE:
        raise HTTPException(status_code=403, detail="Incorrect passphrase")

    # auth_request = requests.Request()
    # signing_credentials = compute_engine.IDTokenCredentials(
    #     auth_request,
    #     "",
    #     service_account_email="storager@taigaishida-217622.iam.gserviceaccount.com",
    # )
    #
    signing_credentials, project = google.auth.default()

    # Generate a signed URL for uploading a file
    # blob = client.bucket(IMAGE_BUCKET).blob(os.path.join(IMAGE_PREFIX, item.filename))
    blob = client.bucket(IMAGE_BUCKET).blob(os.path.join("dev/staging", item.filename))
    url = blob.generate_signed_url(
        version="v4",
        expiration=timedelta(minutes=15),
        credentials=signing_credentials,
        method="PUT",
        content_type=item.content_type,
    )

    return {"uploadUrl": url}
