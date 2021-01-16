from flask import Blueprint, render_template

bp = Blueprint('root', __name__, url_prefix='/')


@bp.route("")
@bp.route("/index")
def index():
    return render_template("root.html")

@bp.route("/gallery")
def gallery():
    return render_template("gallery.html", title="Gallery")

@bp.route("/contact")
def contact():
    return render_template("contact.html", title="Contact")

@bp.route("/about")
def about():
    return render_template("about.html", title="About")