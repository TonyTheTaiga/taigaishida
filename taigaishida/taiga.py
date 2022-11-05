from pathlib import Path
import json
from typing import Optional, List

from flask import Blueprint, render_template
from pydantic import BaseModel

bp = Blueprint("taiga", __name__, url_prefix="/")


class GalleryItem(BaseModel):
    url: str
    name: Optional[str] = "???"


class MetaData(BaseModel):
    gallery: List[GalleryItem]


with open(Path(__file__).parent.resolve() / "static" / "metadata.json", "r") as fp:
    metadata = MetaData(**json.loads(fp.read()))


@bp.route("")
@bp.route("/gallery")
def gallery():
    return render_template("gallery.html", gallery=metadata.gallery)


@bp.route("/about")
def about():
    return render_template("about.html", title="About")


@bp.route("/js")
def js():
    return render_template("js.html", title="JS")
