from flask import Flask

from config import Config
from taigaishida import taiga


def create_app():
    site = Flask(__name__)
    site.config.from_object(Config)

    site.register_blueprint(taiga.bp)

    return site
