from app import website
from flask import render_template

@website.route('/')
@website.route('/index')
def index():
    return render_template('index.html')

@website.route('/picture')
def picture():
    return render_template('gallery.html')
