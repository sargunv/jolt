package dev.sargunv.jolt.oracles;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;

public final class FormatterHarness {
    @FunctionalInterface
    private interface FormatterProfile {
        String format(String input, String filename) throws Exception;
    }

    private record Profile(String name, FormatterProfile formatter) {}

    private record Suite(String name, List<Profile> profiles) {}

    private record SkippedFixture(String profileName, Path inputPath, Exception exception) {}

    private static final List<Suite> SUITES = List.of(
            new Suite(
                    "google-java-format",
                    List.of(
                            new Profile(
                                    "google",
                                    (input, filename) -> GoogleJavaFormatCli.format(
                                            GoogleJavaFormatCli.Profile.GOOGLE, input, filename)),
                            new Profile(
                                    "aosp",
                                    (input, filename) -> GoogleJavaFormatCli.format(
                                            GoogleJavaFormatCli.Profile.AOSP, input, filename)))),
            new Suite("palantir-java-format", List.of(new Profile("palantir", PalantirJavaFormatCli::format))));

    private FormatterHarness() {}

    public static void main(String[] args) throws Exception {
        if (args.length != 1) {
            throw new IllegalArgumentException("usage: FormatterHarness <fixtures-root>");
        }

        var fixturesRoot = Path.of(args[0]);
        var skippedFixtures = new ArrayList<SkippedFixture>();
        for (var suite : SUITES) {
            var inputDir = fixturesRoot.resolve(suite.name()).resolve("input");
            if (!Files.isDirectory(inputDir)) {
                throw new IllegalArgumentException("missing fixture input directory: " + inputDir);
            }
            var inputPaths = fixtureInputs(inputDir);
            System.err.println("materializing " + suite.name() + " outputs from " + inputPaths.size() + " input fixture(s)");
            for (var profile : suite.profiles()) {
                var profileName = suite.name() + "/" + profile.name();
                var written = 0;
                var skipped = 0;
                for (var inputPath : inputPaths) {
                    var outputPath = fixturesRoot
                            .resolve(suite.name())
                            .resolve(profile.name())
                            .resolve(inputPath.getFileName());
                    try {
                        materialize(profile, inputPath, outputPath);
                        written++;
                    } catch (Exception exception) {
                        skippedFixtures.add(new SkippedFixture(profileName, inputPath, exception));
                        skipped++;
                    }
                }
                System.err.println("materialized " + written + " " + profileName + " output fixture(s)"
                        + skippedSuffix(skipped));
            }
        }
        if (!skippedFixtures.isEmpty()) {
            System.err.println("skipped " + skippedFixtures.size() + " oracle fixture(s):");
            for (var skippedFixture : skippedFixtures) {
                System.err.println(
                        "- " + skippedFixture.profileName() + " " + skippedFixture.inputPath() + ": "
                                + diagnostic(skippedFixture.exception()));
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
        try (var paths = Files.list(inputDir)) {
            return paths.filter(path -> path.getFileName().toString().endsWith(".java"))
                    .sorted(Comparator.comparing(path -> path.getFileName().toString()))
                    .toList();
        }
    }

    private static void materialize(Profile profile, Path inputPath, Path outputPath) throws Exception {
        var input = Files.readString(inputPath, StandardCharsets.UTF_8);
        var output = profile.formatter().format(input, inputPath.getFileName().toString());
        var parent = outputPath.getParent();
        if (parent != null) {
            Files.createDirectories(parent);
        }
        Files.writeString(outputPath, output, StandardCharsets.UTF_8);
    }
}
