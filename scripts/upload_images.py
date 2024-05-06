from argparse import ArgumentParser
from concurrent.futures import ThreadPoolExecutor
from functools import lru_cache
from pathlib import Path

from google.cloud import storage
from tqdm import tqdm


@lru_cache()
def get_client() -> storage.Client:
    print(f"creating client")
    return storage.Client()


def main(image_dir: Path, bucket: str):
    if not image_dir.exists():
        raise ValueError(f"{image_dir} doesn't exist")

    client = get_client()

    bucket, prefix = bucket.split("/", maxsplit=1)

    hashes = []

    for blob in client.list_blobs(bucket, prefix=prefix):
        hashes.append(blob.md5_hash)

    print(hashes)

    for item in [
        *image_dir.rglob("*.heic"),
        *image_dir.rglob("*.jpg"),
        *image_dir.rglob("*.png"),
    ]:
        print(item)


if __name__ == "__main__":
    parser = ArgumentParser()
    parser.add_argument("--image_dir", type=Path)

    args = parser.parse_args()
    print(args)
    main(args.image_dir, "taiga-ishida-public/dev/new_tmp")
