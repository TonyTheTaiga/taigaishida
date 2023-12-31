#!/usr/bin/env python3

import argparse

from google.cloud import datastore, storage
import exifread


def create_entity(kind: str, name: str, data: dict, client=None):
    if client is None:
        client = datastore.Client()

    entity = datastore.Entity(client.key(kind, name))
    entity.update(data)
    client.put(entity)


def read_exif(image) -> dict:
    with open(image, "rb") as fp:
        tags = exifread.process_file(fp, details=False)

    # Might need to gracefully handle values that may not cast into strings nicely
    tags = {k.split(" ")[-1]: str(v) for k, v in tags.items()}
    return tags


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--image", type=str, required=True)
    args = parser.parse_args()
    exif = read_exif(args.image)
    print(exif)
