from flask import Flask
from config import Config

website = Flask(__name__)
website.config.from_object(Config)

from app import routes