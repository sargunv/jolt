import tempfile
import unittest
from pathlib import Path

from tools.bench import bench
from tools.corpora import CORPORA


class BenchmarkCorpusTests(unittest.TestCase):
    def test_java_only_tools_are_not_applied_to_kotlin(self) -> None:
        selected = tuple(bench.TOOLS)

        self.assertEqual(
            bench.applicable_tools(CORPORA["kotlin-realistic"], selected),
            ("jolt", "dprint-jolt"),
        )
        self.assertEqual(
            bench.applicable_tools(CORPORA["realistic"], selected),
            selected,
        )

    def test_kotlin_corpus_includes_source_and_script_files(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Source.kt").touch()
            (root / "Build.kts").touch()
            (root / "Ignored.java").touch()

            files = CORPORA["kotlin-realistic"].files(root)

        self.assertEqual(
            [path.name for path in files], ["Build.kts", "Source.kt"]
        )

    def test_modified_file_count_rejects_missing_files(self) -> None:
        corpus = CORPORA["realistic"]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            baseline = root / "baseline"
            formatted = root / "formatted"
            baseline.mkdir()
            formatted.mkdir()
            (baseline / "Present.java").touch()

            with self.assertRaisesRegex(
                RuntimeError, "formatted corpus file set changed"
            ):
                bench.count_modified_files(corpus, baseline, formatted)


if __name__ == "__main__":
    unittest.main()
