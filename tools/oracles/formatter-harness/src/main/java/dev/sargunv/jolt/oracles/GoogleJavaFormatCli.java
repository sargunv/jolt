package dev.sargunv.jolt.oracles;

import com.google.googlejavaformat.java.Main;
import java.io.ByteArrayInputStream;
import java.io.PrintWriter;
import java.io.StringWriter;
import java.nio.charset.StandardCharsets;

public final class GoogleJavaFormatCli {
    public enum Profile {
        GOOGLE,
        AOSP
    }

    private GoogleJavaFormatCli() {}

    public static String format(Profile profile, String input, String filename) throws Exception {
        var output = new StringWriter();
        var errors = new StringWriter();
        var formatter = new Main(
                new PrintWriter(output, true),
                new PrintWriter(errors, true),
                new ByteArrayInputStream(input.getBytes(StandardCharsets.UTF_8)));
        var exitCode = formatter.format(args(profile, filename));
        if (exitCode != 0) {
            throw new IllegalStateException(errors.toString());
        }
        return output.toString();
    }

    private static String[] args(Profile profile, String filename) {
        return switch (profile) {
            case GOOGLE -> new String[] {
                "--skip-removing-unused-imports",
                "--skip-javadoc-formatting",
                "--assume-filename",
                filename,
                "-",
            };
            case AOSP -> new String[] {
                "--aosp",
                "--skip-removing-unused-imports",
                "--skip-javadoc-formatting",
                "--assume-filename",
                filename,
                "-",
            };
        };
    }
}
