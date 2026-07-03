package dev.sargunv.jolt.fixtures;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;

public final class FormatterHarness {
  @FunctionalInterface
  private interface FormatterReference {
    String format(String input, String filename) throws Exception;
  }

  private record Reference(String name, FormatterReference formatter) {
  }

  private record Suite(String name, List<Reference> references) {
  }

  private record SkippedFixture(
    String referenceName,
    Path inputPath,
    Exception exception
  ) {
  }

  private static final List<Suite> SUITES =
    List.of(
      new Suite(
        "google-java-format",
        List.of(
          new Reference(
            "google",
            (input, filename) -> GoogleJavaFormatCli.format(
              GoogleJavaFormatCli.ReferenceMode.GOOGLE,
              input,
              filename
            )
          ),
          new Reference(
            "aosp",
            (input, filename) -> GoogleJavaFormatCli.format(
              GoogleJavaFormatCli.ReferenceMode.AOSP,
              input,
              filename
            )
          )
        )
      ),
      new Suite(
        "palantir-java-format",
        List.of(new Reference("palantir", PalantirJavaFormatCli::format))
      )
    );

  private FormatterHarness() {
  }

  public static void main(String[] args) throws Exception {
    if (args.length != 1) {
      throw new IllegalArgumentException(
        "usage: FormatterHarness <fixtures-root>"
      );
    }

    var fixturesRoot = Path.of(args[0]);
    var skippedFixtures = new ArrayList<SkippedFixture>();
    for (var suite : SUITES) {
      var inputDir = fixturesRoot.resolve(suite.name()).resolve("input");
      if (!Files.isDirectory(inputDir)) {
        throw new IllegalArgumentException(
          "missing fixture input directory: " + inputDir
        );
      }
      var inputPaths = fixtureInputs(inputDir);
      System.err
        .println(
          "materializing "
            + suite.name()
            + " outputs from "
            + inputPaths.size()
            + " input fixture(s)"
        );
      for (var reference : suite.references()) {
        var referenceName = suite.name() + "/" + reference.name();
        var written = 0;
        var skipped = 0;
        for (var inputPath : inputPaths) {
          var outputPath =
            fixturesRoot.resolve(suite.name())
              .resolve(reference.name())
              .resolve(inputPath.getFileName());
          try {
            materialize(reference, inputPath, outputPath);
            written++;
          } catch (Exception exception) {
            skippedFixtures.add(
              new SkippedFixture(referenceName, inputPath, exception)
            );
            skipped++;
          }
        }
        System.err
          .println(
            "materialized "
              + written
              + " "
              + referenceName
              + " output fixture(s)"
              + skippedSuffix(skipped)
          );
      }
    }
    if (!skippedFixtures.isEmpty()) {
      System.err.println("skipped " + skippedFixtures.size() + " fixture(s):");
      for (var skippedFixture : skippedFixtures) {
        System.err
          .println(
            "- "
              + skippedFixture.referenceName()
              + " "
              + skippedFixture.inputPath()
              + ": "
              + diagnostic(skippedFixture.exception())
          );
      }
    }
  }

  private static String skippedSuffix(int skipped) {
    return skipped == 0 ? "" : ", skipped " + skipped;
  }

  private static String diagnostic(Exception exception) {
    var message = exception.getMessage();
    if (message == null || message.isBlank()) {
      return exception.getClass().getName();
    }
    return message.strip().replace('\n', ' ');
  }

  private static List<Path> fixtureInputs(Path inputDir) throws Exception {
    try (
      var paths = Files.list(inputDir)
    ) {
      return paths.filter(
        path -> path.getFileName().toString().endsWith(".java")
      )
        .sorted(Comparator.comparing(path -> path.getFileName().toString()))
        .toList();
    }
  }

  private static void materialize(
    Reference reference,
    Path inputPath,
    Path outputPath
  )
    throws Exception {
    var input = Files.readString(inputPath, StandardCharsets.UTF_8);
    var output =
      reference.formatter().format(input, inputPath.getFileName().toString());
    var parent = outputPath.getParent();
    if (parent != null) {
      Files.createDirectories(parent);
    }
    Files.writeString(outputPath, output, StandardCharsets.UTF_8);
  }
}
