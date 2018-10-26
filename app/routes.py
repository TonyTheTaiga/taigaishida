from app import app
from flask import render_template
from app.forms import ContactForm


@app.route("/")
@app.route("/index")
def index():
    form = ContactForm()
    return render_template("index.html")


@app.route("/picture")
def picture():
    return render_template("gallery.html", title="Gallery")

