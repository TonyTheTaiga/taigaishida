#!/usr/bin/env python3

import os
import argparse
from pathlib import Path

import tqdm
from google.cloud import datastore, storage
import exifread


def create_entity(kind: str, data: dict, client=None) -> datastore.Entity:
    if client is None:
        client = datastore.Client()

    entity = datastore.Entity(client.key(kind))
    entity.update(data)
    return entity


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
    path = Path(args.image)
    exifs = []
    ds_client = datastore.Client()
    s_client = storage.Client()

    if path.is_dir():
        for image in path.glob("*"):
            if image.name.startswith("._") or image.name.endswith(".MOV"):
                continue

            exif = read_exif(image.as_posix())
            if not exif:
                continue

            exifs.append((image.absolute().as_posix(), exif))
    else:
        exif = read_exif(path.as_posix())
        exifs.append((path.absolute().as_posix(), exif))

    entities = []
    for im, exif in tqdm.tqdm(exifs):
        name = os.path.basename(im)
        gs_uri = f"gs://taiga-private/dev/tmp/{name}"
        blob = storage.Blob.from_string(gs_uri, client=s_client)
        blob.upload_from_filename(im)
        entity_data = {**exif, "uri": gs_uri}
        entity = create_entity("Image", exif, ds_client)
        entities.append(entity)

    ds_client.put_multi(entities)
