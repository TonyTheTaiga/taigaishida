from typing import List, Optional
from pathlib import Path
import json

from fastapi import FastAPI, Request
from fastapi.responses import HTMLResponse
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates
from pydantic import BaseModel

app = FastAPI()

app.mount("/static", StaticFiles(directory=Path(__file__).parent.resolve() / "static"), name="static")
templates = Jinja2Templates(directory=Path(__file__).parent.resolve() / "templates")


class GalleryItem(BaseModel):
    url: str
    name: Optional[str] = "???"


class MetaData(BaseModel):
    gallery: List[GalleryItem]


with open(Path(__file__).parent.resolve() / "static" / "metadata.json", "r") as fp:
    metadata = MetaData(**json.loads(fp.read()))


@app.get("/gallery", response_class=HTMLResponse)
@app.get("/", response_class=HTMLResponse)
async def gallery(request: Request):
    return templates.TemplateResponse("gallery.html", {"request": request, "gallery": metadata.gallery})


@app.get("/about", response_class=HTMLResponse)
async def about(request: Request):
    return templates.TemplateResponse("about.html", {"request": request})
