#!/usr/bin/env python3

import os
import argparse
from pathlib import Path
from datetime import datetime

import tqdm
from google.cloud import datastore, storage
import exifread


def dms_to_decimal(dms):
    """
    Convert a DMS (Degrees, Minutes, Seconds) tuple to decimal degrees.

    Args:
    dms (tuple): A tuple representing the DMS coordinates.
                 For example, (degrees, minutes, seconds_as_fraction).

    Returns:
    float: The decimal degree representation of the DMS input.
    """

    # Unpack the degrees, minutes, and seconds
    degrees, minutes, seconds_fraction = dms.replace(",", "").lstrip("[").rstrip("]").split(" ")
    # Convert the seconds fraction if it's a string fraction
    if isinstance(seconds_fraction, str) and "/" in seconds_fraction:
        numerator, denominator = map(float, seconds_fraction.split("/"))
        seconds_fraction = numerator / denominator

    # Convert to decimal degrees
    decimal_degrees = float(degrees) + (float(minutes) / 60) + (seconds_fraction / 3600)

    return decimal_degrees


def convert_datetime_to_iso_format(datetime_str):
    # Parse the original datetime string
    dt = datetime.strptime(datetime_str, "%Y:%m:%d %H:%M:%S")

    # Convert it to the ISO format with milliseconds and 'Z' (for UTC)
    formatted_datetime = dt.strftime("%Y-%m-%dT%H:%M:%S.000Z")

    return formatted_datetime


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

    tags["created"] = convert_datetime_to_iso_format(tags["DateTime"])
    try:
        tags[
            "latlong"
        ] = f'{dms_to_decimal(tags["GPSLatitude"]):.5f} {tags["GPSLatitudeRef"]}, {dms_to_decimal(tags["GPSLongitude"]):.5f} {tags["GPSLongitudeRef"]}'
    except Exception as e:
        print(e)
        pass

    # Convert lat long to a city, Earth if unknown
    # tags['city'] = ...

    return tags


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--image", type=str, required=True)
    args = parser.parse_args()
    path = Path(args.image)
    exifs = []
    ds_client = datastore.Client(database="")
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
        gs_uri = f"gs://taiga-ishida-public/dev/tmp/{name}"
        blob = storage.Blob.from_string(gs_uri, client=s_client)
        # blob.upload_from_filename(im)
        entity_data = {**exif, "uri": blob.public_url, "name": os.path.basename(im)}
        entity = create_entity("Image", entity_data, ds_client)
        entities.append(entity)

    # ds_client.put_multi(entities)
