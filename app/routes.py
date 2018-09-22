from app import app
from flask import render_template

@app.route('/')
@app.route('/index')
def index():
    return render_template('test.html')

@app.route('/picture')
def picture():
    return render_template('test.html')