package dev.sargunv.jolt.fixtures;

import com.palantir.javaformat.java.Main;
import java.io.ByteArrayInputStream;
import java.io.PrintWriter;
import java.io.StringWriter;
import java.nio.charset.StandardCharsets;

public final class PalantirJavaFormatCli {
  private PalantirJavaFormatCli() {
  }

  public static String format(String input, String filename) throws Exception {
    var output = new StringWriter();
    var errors = new StringWriter();
    var formatter =
      new Main(
        new PrintWriter(output, true),
        new PrintWriter(errors, true),
        new ByteArrayInputStream(input.getBytes(StandardCharsets.UTF_8))
      );
    var exitCode =
      formatter.format(
        "--palantir",
        "--skip-removing-unused-imports",
        "--assume-filename",
        filename,
        "-"
      );
    if (exitCode != 0) {
      throw new IllegalStateException(errors.toString());
    }
    return output.toString();
  }
}
