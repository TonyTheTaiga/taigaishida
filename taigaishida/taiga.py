from flask import Blueprint, render_template, request

bp = Blueprint("taiga", __name__, url_prefix="/")


@bp.route("")
@bp.route("/index")
def index():
    return render_template("index.html")


@bp.route("/about")
def about():
    return render_template("about.html", title="About")
