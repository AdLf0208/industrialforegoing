#!/usr/bin/env python3
"""Parse `/industrialforegoing info`'s line into shell assignments.

`Water Condensator at [202, 101, 0]: 4000/10000 FE, water 200/16000 mB,
sources 2, progress 10/100, facing west, t=264`

Prints `ENERGY=… FE_CAP=… WATER=… WATER_CAP=… SOURCES=… PROGRESS=…
PROGRESS_MAX=… FACING=… T=…` (or `ok=0`), for the e2e to `eval`.
"""

from __future__ import annotations

import re
import sys

LINE = re.compile(
    r"^Water Condensator at \[.*?\]: (?P<energy>-?\d+)/(?P<fe_cap>\d+) FE, "
    r"water (?P<water>-?\d+)/(?P<water_cap>\d+) mB, "
    r"sources (?P<sources>\d+), progress (?P<progress>-?\d+)/(?P<progress_max>\d+), "
    r"facing (?P<facing>[a-z_]+), t=(?P<t>\d+)$"
)


def main() -> None:
    line = sys.argv[1] if len(sys.argv) > 1 else ""
    match = LINE.match(line)
    if not match:
        print("ok=0")
        return
    print("ok=1")
    print(f"ENERGY={match['energy']}")
    print(f"FE_CAP={match['fe_cap']}")
    print(f"WATER={match['water']}")
    print(f"WATER_CAP={match['water_cap']}")
    print(f"SOURCES={match['sources']}")
    print(f"PROGRESS={match['progress']}")
    print(f"PROGRESS_MAX={match['progress_max']}")
    print(f"FACING={match['facing']}")
    print(f"T={match['t']}")


if __name__ == "__main__":
    main()
