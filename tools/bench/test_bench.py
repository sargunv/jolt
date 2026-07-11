import tempfile
import unittest
from pathlib import Path

from tools.bench import bench
from tools.pgo import build as pgo


class BenchmarkCorpusTests(unittest.TestCase):
    def test_java_only_tools_are_not_applied_to_kotlin(self) -> None:
        selected = tuple(bench.TOOLS)

        self.assertEqual(
            bench.applicable_tools(bench.CORPORA["kotlin-realistic"], selected),
            ("jolt", "dprint-jolt"),
        )
        self.assertEqual(
            bench.applicable_tools(bench.CORPORA["realistic"], selected),
            selected,
        )

    def test_kotlin_corpus_includes_source_and_script_files(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Source.kt").touch()
            (root / "Build.kts").touch()
            (root / "Ignored.java").touch()

            files = bench.corpus_files(bench.CORPORA["kotlin-realistic"], root)

        self.assertEqual(
            [path.name for path in files], ["Build.kts", "Source.kt"]
        )

    def test_pgo_corpora_match_benchmark_sources_and_extensions(self) -> None:
        self.assertEqual(set(pgo.CORPORA), set(bench.CORPORA))
        for name, corpus in bench.CORPORA.items():
            source, extensions, excluded = pgo.CORPORA[name]
            self.assertEqual(source, corpus["source"])
            self.assertEqual(extensions, corpus["extensions"])
            self.assertEqual(excluded, set(corpus["exclude"]))


if __name__ == "__main__":
    unittest.main()
