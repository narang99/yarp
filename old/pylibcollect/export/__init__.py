from pathlib import Path
from pylibcollect.export.closure import generate_mac_closure
from pylibcollect.export.pkg.main import export_py_app
import json

from pylibcollect.types import PyLibCollectPayload


def make_py_app(pylibcollect_payload: PyLibCollectPayload, out_dir: Path) -> None:
    generate_mac_closure()