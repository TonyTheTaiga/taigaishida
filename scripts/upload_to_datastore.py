#!/usr/bin/env python3

from functools import lru_cache
from collections.abc import Iterable
from contextlib import ExitStack
from concurrent.futures import ProcessPoolExecutor, as_completed
import json
import os
import argparse
from pathlib import Path
from datetime import datetime
from typing import NamedTuple

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
        tags = exifread.process_file(fp, details=False)  # pyright: ignore

    # Might need to gracefully handle values that may not cast into strings nicely
    tags = {k.split(" ")[-1]: str(v) for k, v in tags.items()}
    tags["created"] = convert_datetime_to_iso_format(tags["DateTime"])

    print(json.dumps(tags, indent=4))

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


class ExifData(NamedTuple):
    data: dict
    image: str


def exif_generator(path_or_image: Path):
    if path_or_image.is_dir():
        for image in path_or_image.glob("*"):
            if image.name.startswith(".") or image.name.endswith(".MOV"):
                continue

            exif = read_exif(image.as_posix())
            if not exif:
                continue

            yield ExifData(data=exif, image=image.absolute().as_posix())
    else:
        exif = read_exif(path_or_image.as_posix())
        yield ExifData(data=exif, image=path_or_image.absolute().as_posix())


@lru_cache()
def get_client():
    print(f"creating client")
    return storage.Client()


@lru_cache()
def get_ds_client():
    print(f"creating ds client")
    return datastore.Client()


def _process(data: ExifData):
    s_client = get_client()
    ds_client = get_ds_client()

    name = os.path.basename(data.image)
    gs_uri = f"gs://taiga-ishida-public/dev/tmp/{name}"
    blob = storage.Blob.from_string(gs_uri, client=s_client)
    blob.upload_from_filename(data.image)

    # resize image

    entity_data = {
        **data.data,
        "uri": blob.public_url,
        "name": os.path.basename(data.image),
    }
    entity = create_entity("Image", entity_data, ds_client)
    return entity


def process_data(data: Iterable[ExifData], num_files=None):
    with ExitStack() as stack:
        executor = stack.enter_context(ProcessPoolExecutor(max_workers=8))
        process_bar = stack.enter_context(tqdm.tqdm(total=num_files))
        futures = [executor.submit(_process, _data) for _data in data]

        for future in as_completed(futures):
            process_bar.update(1)
            yield future.result()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--image", type=str, required=True)
    args = parser.parse_args()
    path = Path(args.image)
    ds_client = datastore.Client(database="", project="taigaishida-217622")
    s_client = storage.Client()
    entities = list(process_data(exif_generator(path)))
    # get_ds_client().put_multi(entities)
