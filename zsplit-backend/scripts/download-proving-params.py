#!/usr/bin/env python3
"""Fetch public prover files and verify the hashes pinned in zcash_proofs 0.31.0."""
import hashlib
from pathlib import Path
from urllib.request import urlopen

PARAMS = {
    "sapling-spend.params": (47958396, "8270785a1a0d0bc77196f000ee6d221c9c9894f55307bd9357c3f0105d31ca63991ab91324160d8f53e2bbd3c2633a6eb8bdf5205d822e7f3f73edac51b2b70c"),
    "sapling-output.params": (3592860, "657e3d38dbb5cb5e7dd2970e8b03d69b4787dd907285b5a7f0790dcc8072f60bf593b32cc2d1c030e00ff5ae64bf84c5c3beb84ddc841d48264b4a171744d028"),
}


def main():
    destination = Path(__file__).resolve().parents[1] / "data" / "params"
    destination.mkdir(parents=True, exist_ok=True)
    for name, (size, expected) in PARAMS.items():
        target = destination / name
        if target.exists():
            if target.stat().st_size != size or hashlib.blake2b(target.read_bytes()).hexdigest() != expected:
                raise RuntimeError(f"Existing {name} failed verification; investigate before replacing it")
            print(f"Verified {name}")
            continue
        partial = target.with_suffix(".download")
        digest = hashlib.blake2b()
        length = 0
        with urlopen(f"https://download.z.cash/downloads/{name}", timeout=60) as response, partial.open("wb") as output:
            while chunk := response.read(1024 * 1024):
                output.write(chunk)
                digest.update(chunk)
                length += len(chunk)
        if length != size or digest.hexdigest() != expected:
            partial.unlink(missing_ok=True)
            raise RuntimeError(f"Downloaded {name} failed SDK integrity verification")
        partial.replace(target)
        print(f"Downloaded and verified {name}")


if __name__ == "__main__":
    main()
