import base64
import colorsys
import json
import logging
import os
import random
from datetime import datetime, timedelta
from functools import lru_cache
from io import BytesIO
from pathlib import Path

import exifread
import google.auth
from colorthief import ColorThief
from fastapi import BackgroundTasks, Depends, FastAPI, HTTPException, Request
from fastapi.responses import HTMLResponse
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates
from google.auth import compute_engine
from google.auth.transport import requests
from google.cloud import datastore, secretmanager, storage
from google.cloud.datastore.query import PropertyFilter
from openai import OpenAI
from PIL import Image, ImageOps
from pillow_heif import register_heif_opener
from pydantic import BaseModel

register_heif_opener()

PUBLIC_BUCKET = "taiga-ishida-public"
PRIVATE_BUCKET = "taiga-ishida-private"
IMAGE_PREFIX = "webp_images"
GCS_API_ROOT = "https://storage.googleapis.com"
CORRECT_PASSPHRASE = "greengrass123"
STORAGE_SA = "storager@taigaishida-217622.iam.gserviceaccount.com"
IMAGE_KIND = "Image"

logger = logging.getLogger("ui")
bg_logger = logging.getLogger("bg")


class Item(BaseModel):
    filename: str
    passphrase: str
    content_type: str


class RegisterImageRequest(BaseModel):
    filename: str


class GalleryItem(BaseModel):
    url: str
    haiku: list[str]
    line1: str
    line2: str
    line3: str
    created: str | None = None
    latlong: str | None = None
    model: str | None = None
    exposure_time: str | None = None
    f_number: str | None = None
    iso: str | None = None
    focal_length: str | None = None
    focal_length_35mm: str | None = None
    md5: str | None = None
    dominant_hue: int | None = None
    dominant_rgb: list[int] | None = None
    saturation: float | None = None
    brightness: float | None = None
    color_category: str | None = None

    @classmethod
    def from_entity(cls, entity: dict) -> "GalleryItem":
        haiku = entity.get("haiku", ["", "", ""])
        return cls(
            url=entity.get("public_url"),
            haiku=haiku,
            line1=haiku[0],
            line2=haiku[1],
            line3=haiku[2],
            created=entity.get("created"),
            latlong=entity.get("latlong"),
            model=entity.get("Model"),
            exposure_time=entity.get("ExposureTime"),
            f_number=entity.get("FNumber"),
            iso=entity.get("ISOSpeedRatings"),
            focal_length=entity.get("FocalLength"),
            focal_length_35mm=entity.get("FocalLengthIn35mmFilm"),
            md5=entity.get("md5"),
            dominant_hue=entity.get("dominant_hue"),
            dominant_rgb=entity.get("dominant_rgb"),
            saturation=entity.get("saturation"),
            brightness=entity.get("brightness"),
            color_category=entity.get("color_category"),
        )


@lru_cache()
def get_client() -> storage.Client:
    logger.debug(f"creating client")
    return storage.Client(project="taigaishida-217622")


@lru_cache()
def get_ds_client() -> datastore.Client:
    logger.debug(f"creating ds client")
    return datastore.Client(project="taigaishida-217622")


@lru_cache()
def get_openai_client() -> OpenAI:
    api_key = os.environ.get("OPENAI_API_KEY")
    if not api_key:
        raise RuntimeError("OPENAI_API_KEY not set")

    if api_key.startswith("projects/"):
        secret_manager = secret_manager_client()
        api_key = secret_manager.access_secret_version(
            name=api_key
        ).payload.data.decode("utf-8")

    return OpenAI(api_key=api_key)


@lru_cache()
def secret_manager_client() -> secretmanager.SecretManagerServiceClient:
    return secretmanager.SecretManagerServiceClient()


def get_haiku(b64_image: str):
    client = get_openai_client()
    response = client.chat.completions.create(
        model="gpt-4-turbo",
        messages=[
            {
                "role": "system",
                "content": "Write haikus about the images shown to you. Follow all haiku rules.",
            },
            {
                "role": "system",
                "content": 'Return haikus in json string format with the following schema {"1": "5 syllables", "2": "7 syllables", "3": "5 syallbles"}',
            },
            {
                "role": "user",
                "content": [
                    {"type": "text", "text": "Write a haiku describing this image"},
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": f"data:image/webp;base64,{b64_image}",
                        },
                    },
                ],
            },
        ],
        max_tokens=500,
    )

    content = response.choices[0].message.content
    if not content:
        raise RuntimeError("didn't get a valid response from openAI")

    try:
        message = json.loads(content)  # pyright: ignore
    except json.decoder.JSONDecodeError as e:
        if content.startswith("```json"):
            message = json.loads(
                content.lstrip("```json")
                .rstrip("```")
                .replace("\n", "")
                .replace(" ", "")
            )
        else:
            print(response.choices[0].message.content)
            raise e

    return list(message.values())


def extract_dominant_hue(image_bytes: bytes) -> dict:
    """Extract dominant color information from image bytes"""
    try:
        # Create ColorThief instance from bytes
        color_thief = ColorThief(BytesIO(image_bytes))

        # Get dominant color (RGB)
        dominant_color = color_thief.get_color(quality=1)

        # Convert RGB to HSV to get hue
        r, g, b = [x / 255.0 for x in dominant_color]
        h, s, v = colorsys.rgb_to_hsv(r, g, b)

        # Convert hue to degrees (0-360)
        hue_degrees = int(h * 360)

        # Get color category
        color_category = get_color_category(hue_degrees)

        return {
            "dominant_hue": hue_degrees,
            "dominant_rgb": list(dominant_color),
            "saturation": round(s, 2),
            "brightness": round(v, 2),
            "color_category": color_category,
        }
    except Exception as e:
        logger.warning(f"Could not extract color data: {e}")
        return {
            "dominant_hue": None,
            "dominant_rgb": None,
            "saturation": None,
            "brightness": None,
            "color_category": None,
        }


def get_color_category(hue: int) -> str:
    """Group hues into artistic color families"""
    if hue < 15 or hue > 345:
        return "red"
    elif hue < 45:
        return "orange"
    elif hue < 75:
        return "yellow"
    elif hue < 150:
        return "green"
    elif hue < 210:
        return "blue"
    elif hue < 270:
        return "purple"
    else:
        return "pink"


def get_gallery_data():
    ds_client = get_ds_client()
    entities = get_entities("Image", ds_client)
    gallery = [GalleryItem.from_entity(e) for e in entities]

    # Sort by brightness: light to dark (highest to lowest brightness)
    gallery.sort(key=lambda x: x.brightness or 0, reverse=True)

    return gallery


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


def _convert_datetime_to_iso_format(datetime_str):
    # Parse the original datetime string
    dt = datetime.strptime(datetime_str, "%Y:%m:%d %H:%M:%S")

    # Convert it to the ISO format with milliseconds and 'Z' (for UTC)
    formatted_datetime = dt.strftime("%Y-%m-%dT%H:%M:%S.000Z")

    return formatted_datetime


def read_exif(image_buf) -> dict:
    tags = exifread.process_file(image_buf, details=False)  # pyright: ignore

    # Might need to gracefully handle values that may not cast into strings nicely
    tags = {k.split(" ")[-1]: str(v).strip() for k, v in tags.items()}
    tags["created"] = _convert_datetime_to_iso_format(tags["DateTime"])

    try:
        tags["latlong"] = (
            f"{dms_to_decimal(tags['GPSLatitude']):.5f} {tags['GPSLatitudeRef']}, {dms_to_decimal(tags['GPSLongitude']):.5f} {tags['GPSLongitudeRef']}"
        )
    except Exception:
        pass

    # Convert lat long to a city, Earth if unknown
    # tags['city'] = ...

    return tags


def create_entity(kind: str, data: dict, client=None) -> datastore.Entity:
    entity = datastore.Entity(client.key(kind))  # pyright: ignore
    entity.update(data)
    return entity


def get_blob(src, client) -> storage.Blob:
    return storage.Blob.from_string(src, client=client)


def check_entity_exists(hash: str, client: datastore.Client):
    q = client.query(kind=IMAGE_KIND)
    q.add_filter(filter=PropertyFilter("md5", "=", hash))
    if len(list(q.fetch())) > 0:
        return True

    return False


def convert_to_webp(image: Image.Image, quality=90) -> bytes:
    image = image.convert("RGB")
    image_bytes = BytesIO()
    image.save(image_bytes, "WEBP", quality=quality)
    return image_bytes.getvalue()


def read_image_bytes(data: BytesIO) -> Image.Image:
    return Image.open(data)


def upload(
    data: bytes, url: str, client: storage.Client, content_type: str
) -> storage.Blob:
    blob = storage.Blob.from_string(url, client=client)
    blob.upload_from_string(data, content_type=content_type)

    return blob


def register_image_bg(request: RegisterImageRequest):
    bg_logger.info(f"processing {request}")
    client = get_client()
    ds_client = get_ds_client()
    src = f"gs://{PRIVATE_BUCKET}/staging/{request.filename}"
    blob = get_blob(src, client)

    if not blob.exists():
        bg_logger.warning(f"blob doesn't exist")
        return

    blob.reload()
    hash = str(blob.md5_hash)

    if check_entity_exists(hash, ds_client):
        blob.delete()
        bg_logger.warning(f"image with md5 already exists")
        return

    image_data = BytesIO(blob.open("rb").read())  # pyright: ignore
    image = read_image_bytes(image_data)
    try:
        image = ImageOps.exif_transpose(image)
    except:
        pass

    webp_image = convert_to_webp(image, quality=75)  # pyright: ignore
    new_filename = request.filename.split(".")[0] + ".webp"
    webp_blob = upload(
        webp_image,
        f"gs://{PUBLIC_BUCKET}/{IMAGE_PREFIX}/{new_filename}",
        client,
        "image/webp",
    )
    try:
        exif_data = read_exif(image_data)
    except:
        exif_data = {}

    # Extract color information
    color_data = extract_dominant_hue(webp_image)

    b64_image = base64.b64encode(webp_image).decode("utf-8")
    haiku_lines = get_haiku(b64_image)
    data = {
        "name": request.filename,
        "md5": hash,
        "haiku": haiku_lines,
        "public_url": webp_blob.public_url,
        **exif_data,
        **color_data,
    }
    entity = create_entity(IMAGE_KIND, data, ds_client)
    ds_client.put(entity)


def get_entities(kind, client: datastore.Client):
    query = client.query(kind=kind)
    return list(query.fetch())


def build_app() -> FastAPI:
    logging.basicConfig(level=logging.INFO)

    app = FastAPI()

    app.mount(
        "/static",
        StaticFiles(directory=Path(__file__).parent.resolve() / "static"),
        name="static",
    )
    templates = Jinja2Templates(directory=Path(__file__).parent.resolve() / "templates")

    @app.post("/register-image", status_code=201)
    def register_image(
        request: RegisterImageRequest, background_tasks: BackgroundTasks
    ):
        background_tasks.add_task(register_image_bg, request)

    @app.post("/get-upload-url")
    def get_upload_url(item: Item):
        if item.passphrase != CORRECT_PASSPHRASE:
            raise HTTPException(status_code=403, detail="Incorrect passphrase")

        auth_request = requests.Request()
        signing_credentials = compute_engine.IDTokenCredentials(
            auth_request,
            "",
            service_account_email=STORAGE_SA,
        )

        # For dev uncomment below and commenty above
        # signing_credentials, project = google.auth.default()  # pyright: ignore

        client = get_client()
        blob = client.bucket(PRIVATE_BUCKET).blob(
            os.path.join("staging", item.filename)
        )
        url = blob.generate_signed_url(
            version="v4",
            expiration=timedelta(minutes=5),
            credentials=signing_credentials,
            method="PUT",
            content_type=item.content_type,
        )

        return {"uploadUrl": url}

    @app.get("/", response_class=HTMLResponse)
    async def home(request: Request):
        gallery = get_gallery_data()
        if len(gallery) == 0:
            return "Empty Gallery!"

        return templates.TemplateResponse(
            "home.html",
            {"request": request, "gallery": gallery},
        )

    @app.get("/upload", response_class=HTMLResponse)
    async def upload(request: Request):
        return templates.TemplateResponse("upload.html", {"request": request})

    return app
