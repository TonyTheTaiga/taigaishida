import os


class Config(object):
    SECRET_KEY = os.environ.get(
        "SECRET_KEY") or "ff6e980f78376f7f7362d1ba14251b89"

