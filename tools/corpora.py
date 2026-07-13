"""Shared benchmark and profile-training corpus definitions."""

import fnmatch
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
IMPORTS = ROOT / "tools/import/.imports"


@dataclass(frozen=True)
class Corpus:
    language: str
    extensions: tuple[str, ...]
    source: Path
    exclude: tuple[str, ...] = ()

    def files(self, directory: Path | None = None) -> list[Path]:
        root = directory or self.source
        return sorted(
            path
            for extension in self.extensions
            for path in root.rglob(f"*{extension}")
            if not self.is_excluded(path.relative_to(root))
        )

    def is_excluded(self, path: Path) -> bool:
        posix = path.as_posix()
        return any(
            fnmatch.fnmatchcase(posix, pattern) for pattern in self.exclude
        )


CORPORA = {
    "adversarial": Corpus(
        language="java",
        extensions=(".java",),
        source=IMPORTS / "google-java-format/input",
        # Intentionally invalid upstream Java.
        exclude=("B26952926.java",),
    ),
    "realistic": Corpus(
        language="java",
        extensions=(".java",),
        source=IMPORTS / "spring-framework",
    ),
    "kotlin-realistic": Corpus(
        language="kotlin",
        extensions=(".kt", ".kts"),
        source=IMPORTS / "maplibre-compose/source",
    ),
}

REALISTIC_CORPUS_KEYS = ("realistic", "kotlin-realistic")
