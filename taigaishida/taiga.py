import logging
import math
import os
from collections import defaultdict
from datetime import timedelta
from pathlib import Path
from uuid import uuid4

from fastapi import Cookie, FastAPI, HTTPException, Request
from fastapi.responses import HTMLResponse
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates
from google.auth import compute_engine
from google.auth.transport import requests
from google.cloud import datastore, storage
from pydantic import BaseModel

IMAGE_BUCKET = "taiga-ishida-public"
IMAGE_PREFIX = "webp_images"
GCS_API_ROOT = "https://storage.googleapis.com"
CORRECT_PASSPHRASE = "greengrass123"

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("ui")

ds_client = datastore.Client()
gcs_client = storage.Client()

app = FastAPI()

session_ids = defaultdict(list)

app.mount(
    "/static",
    StaticFiles(directory=Path(__file__).parent.resolve() / "static"),
    name="static",
)
templates = Jinja2Templates(directory=Path(__file__).parent.resolve() / "templates")


@app.get("/", response_class=HTMLResponse)
@app.get("/index", response_class=HTMLResponse)
def index(request: Request, session_id=Cookie(None)):
    response = templates.TemplateResponse("2024.html", {"request": request})
    response.set_cookie(key="session_id", value=uuid4().hex, httponly=True)
    return response


@app.get("/about", response_class=HTMLResponse)
async def about(request: Request):
    return templates.TemplateResponse("about.html", {"request": request})


@app.get("/table", response_class=HTMLResponse)
async def table(request: Request, session_id=Cookie(None)):
    logger.info(f"session_id: {session_id}")

    limit = 10
    query = ds_client.query(kind="Image")
    count_query = ds_client.aggregation_query(query).count()
    total_images = next(count_query.fetch())[0].value
    total_pages = math.ceil(total_images / limit)

    query.order = ["-created"]
    start_cursor = session_ids[session_id][-1] if session_id in session_ids else None
    query_iterator = query.fetch(limit=limit, start_cursor=start_cursor)
    images = list(next(query_iterator.pages))
    if len(images) < 10:
        images += [None] * (10 - (len(images)))

    session_ids[session_id].append(query_iterator.next_page_token)
    print(session_ids[session_id])
    response = templates.TemplateResponse(
        "table.html",
        {
            "request": request,
            "images": images,
            "total_pages": total_pages,
            "current_page": session_ids[session_id].index(query_iterator.next_page_token) + 1,
        },
    )
    return response


@app.get("/upload", response_class=HTMLResponse)
async def upload(request: Request):
    return templates.TemplateResponse("upload.html", {"request": request})


class Asset(BaseModel):
    filename: str
    passphrase: str
    content_type: str


@app.post("/get-upload-url")
def get_upload_url(asset: Asset):
    if asset.passphrase != CORRECT_PASSPHRASE:
        raise HTTPException(status_code=403, detail="Incorrect passphrase")

    logger.info(f"SA {gcs_client._credentials.service_account_email}")  # type: ignore

    auth_request = requests.Request()
    signing_credentials = compute_engine.IDTokenCredentials(
        auth_request,
        "",
        service_account_email="storager@taigaishida-217622.iam.gserviceaccount.com",
    )

    # Generate a signed URL for uploading a file
    blob = gcs_client.bucket(IMAGE_BUCKET).blob(
        os.path.join(IMAGE_PREFIX, asset.filename)
    )
    url = blob.generate_signed_url(
        version="v4",
        expiration=timedelta(minutes=15),
        credentials=signing_credentials,
        method="PUT",
        content_type=asset.content_type,
    )

    return {"uploadUrl": url}
