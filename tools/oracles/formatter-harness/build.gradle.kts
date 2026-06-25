plugins {
  application
}

java {
  toolchain {
    languageVersion.set(JavaLanguageVersion.of(25))
  }
}

dependencies {
  implementation("com.google.googlejavaformat:google-java-format:1.35.0")
  implementation("com.palantir.javaformat:palantir-java-format:2.94.0")
  implementation("com.facebook:ktfmt:0.64")
}

dependencyLocking {
  lockAllConfigurations()
}

application {
  mainClass.set("dev.sargunv.jolt.oracles.FormatterHarness")
  // google-java-format and palantir-java-format use javac internals on the classpath;
  // Java 25 keeps those packages encapsulated unless the harness exports them.
  applicationDefaultJvmArgs = listOf(
    "--add-exports=jdk.compiler/com.sun.tools.javac.file=ALL-UNNAMED",
    "--add-exports=jdk.compiler/com.sun.tools.javac.parser=ALL-UNNAMED",
    "--add-exports=jdk.compiler/com.sun.tools.javac.tree=ALL-UNNAMED",
    "--add-exports=jdk.compiler/com.sun.tools.javac.util=ALL-UNNAMED",
  )
}
