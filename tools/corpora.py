"""Shared benchmark and profile-training corpus definitions."""

import fnmatch
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
IMPORTS = ROOT / "tools/import/.imports"


@dataclass(frozen=True)
class Corpus:
    description: str
    language: str
    extensions: tuple[str, ...]
    source: Path
    exclude: tuple[str, ...] = ()
    tool_exclude: dict[str, tuple[str, ...]] = field(default_factory=dict)

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
        description="google-java-format formatter test inputs",
        language="java",
        extensions=(".java",),
        source=IMPORTS / "google-java-format/input",
        # Intentionally invalid upstream Java.
        exclude=("B26952926.java",),
        tool_exclude={
            # Accepted by google-java-format and Jolt but rejected by
            # prettier-plugin-java 2.10.2.
            "prettier-java": ("B38352414.java",),
        },
    ),
    "realistic": Corpus(
        description="Spring Framework Java sources",
        language="java",
        extensions=(".java",),
        source=IMPORTS / "spring-framework",
    ),
    "kotlin-realistic": Corpus(
        description="MapLibre Compose Kotlin sources",
        language="kotlin",
        extensions=(".kt", ".kts"),
        source=IMPORTS / "maplibre-compose/source",
    ),
}
