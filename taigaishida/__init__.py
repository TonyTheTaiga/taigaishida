from flask import Flask

from config import Config
from taigaishida import root


def create_app():
    site = Flask(__name__)
    site.config.from_object(Config)

    site.register_blueprint(root.bp)

    return site
