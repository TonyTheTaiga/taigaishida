from app import website
from flask import render_template
from app.forms import ContactForm


@website.route("/")
@website.route("/index")
def index():
    form = ContactForm()
    return render_template("index.html")


@website.route("/picture")
def picture():
    return render_template("gallery.html", title="Gallery")
