from pathlib import Path
import json

from flask import Blueprint, render_template
from pydantic import BaseModel

bp = Blueprint("taiga", __name__, url_prefix="/")


class MetaData(BaseModel):
    gallery: list


with open(Path(__file__).parent.resolve() / "static" / "metadata.json", "r") as fp:
    metadata = MetaData(**json.loads(fp.read()))


@bp.route("")
@bp.route("/index")
def index():
    return render_template("index.html", gallery=metadata.gallery)


@bp.route("/about")
def about():
    return render_template("about.html", title="About")
