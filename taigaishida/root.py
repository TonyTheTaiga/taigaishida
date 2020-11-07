from flask import Blueprint, render_template

bp = Blueprint('root', __name__, url_prefix='/')


@bp.route("")
@bp.route("/index")
def index():
    return render_template("root.html")


@bp.route("/picture")
def picture():
    return render_template("gallery.html", title="Gallery")


@bp.route("/contact")
def contact():
    return render_template("contact.html", title="Contact")

@bp.route("/chiaki")
def chiaki():
    return render_template("chiaki.html")