from app import app
from flask import render_template

@app.route('/')
@app.route('/index')
def index():
    return render_template('index.html')

@app.route('/picture')
def picture():
    return render_template('gallery.html')

@app.route('/construction')
def construction():
    return render_template('construction.html')