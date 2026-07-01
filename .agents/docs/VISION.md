# Jolt Vision

## Summary

Jolt is a fast, opinionated JVM and Kotlin Multiplatform project toolchain.

It begins as a collection of high-performance developer tools for Java and
Kotlin projects, likely written in Rust, inspired by the product clarity of
Astral’s Python tools and the shared-engine strategy of Oxc. Its long-term goal
is more ambitious: to become a manifest-first project manager for JVM, Kotlin
Multiplatform, Android, and native-adjacent library projects, gradually reducing
and then replacing the need for hand-authored Gradle in many libraries and
applications.

Jolt does not replace the JDK, Kotlin compiler, Maven Central, Android SDK,
native toolchains, or platform toolchains. It provides a modern project
substrate around them: formatting, linting, dependency resolution, lockfiles,
toolchain pinning, launch configuration, native/platform artifact modeling,
packaging, project explanation, Gradle interop, and eventually direct build
execution for common project shapes.

## Product Thesis

Jolt is a manifest-first JVM and Kotlin Multiplatform project toolchain. It
provides the clarity of `package.json`, `pyproject.toml`, `uv`, and `ruff`,
while preserving compatibility with Maven Central, Kotlin Multiplatform,
Android, native code, and the JDK.

The central product promise:

> A JVM/KMP project should be easy to describe, explain, lock, update, format,
> build, test, package, publish, and run.

Jolt should preserve the power of the JVM ecosystem while replacing accidental
complexity with visible machinery and boring defaults.

## Problem

Modern JVM and KMP projects are powerful but cognitively expensive.

Gradle can express almost anything, but routine project configuration often
requires deep knowledge of plugins, task wiring, configuration cache behavior,
source sets, toolchains, variant resolution, Kotlin Gradle Plugin behavior,
Android Gradle Plugin behavior, and IDE sync quirks.

For many projects, Gradle is used primarily because it is the only practical way
to access JVM, Kotlin, Kotlin Multiplatform, Android, Compose, Maven publishing,
ecosystem plugins, and native packaging conventions. The actual desired project
shape is often much simpler than the build scripts required to express it.

### Project authoring

Build configuration is too programmable for ordinary project structure. Simple
libraries can require complex Gradle scripts, especially when they publish to
Maven Central, target KMP, depend on Android tooling, or include native
artifacts.

Kotlin Gradle scripts are also difficult for tools to analyze reliably. Even
valid `build.gradle.kts` files can produce IDE red underlines, stale
diagnostics, or confusing sync behavior.

### Dependency and package management

Dependency resolution is opaque, especially across Maven POMs, Gradle Module
Metadata, KMP variants, Android AARs, BOMs, platforms, and transitive
dependencies.

Lockfiles and dependency update policies are not first-class enough in everyday
JVM workflows. Developers often want a simple policy, such as “do not update to
artifacts published in the last 14 days unless I override it manually,” without
adopting a full dependency-update rules engine.

### Toolchains and launch configuration

JDK pinning is confusing because several related concerns are often blurred:

- the JDK that runs Gradle,
- the Java toolchain used by `javac`,
- the JDK used to run tests,
- the JDK used by Kotlin/JVM compilation,
- the Kotlin JVM bytecode target,
- Android SDK levels,
- IDE project SDKs,
- and runtime launch flags.

Modern JVM launch behavior adds more complexity. Some projects need classpath
mode, some use module path, and native code may require native access flags,
`add-opens`, or `add-exports`. These concerns should be modeled once and applied
consistently to CLI runs, tests, IDEs, generated Gradle, packaging, and CI.

### Native and platform artifacts

Many JVM, Kotlin Multiplatform, and Android libraries depend on native code
through JNI, FFM, Kotlin/Native interop, Android native libraries, or
platform-specific runtime artifacts.

Today, this is difficult to model cleanly. Library authors often encode platform
reality indirectly through Gradle tasks, classifier artifacts, empty
platform-specific projects, manually packaged shared libraries, Android AAR
conventions, Kotlin/Native interop configuration, custom runtime loaders, and
external build orchestration.

The problem is not only native compilation. Many projects already build native
code outside Gradle successfully. The harder problem is expressing the resulting
artifacts as part of the project’s dependency, packaging, runtime, and
publication model.

Jolt should make native and platform artifacts understandable, reproducible,
publishable, and explainable across JVM, KMP, Android, and future Wasm-adjacent
targets.

### Source quality tools

Formatting, import cleanup, linting, autofix, and project diagnostics are
fragmented across tools. Existing tools are useful, but they are not usually
part of one coherent project model.

### Packaging and distribution

JVM and Android projects have several artifact shapes, each with different
dependency semantics:

- thin library artifacts,
- fat or uber JARs,
- shaded and relocated JARs,
- minimized or obfuscated artifacts,
- Android AARs,
- Android APKs or AABs,
- desktop runtime images,
- native-image outputs,
- KMP publication families.

These are not just output formats. They affect dependency scopes, runtime
behavior, native loading, publication metadata, and compatibility.

### Build replacement

Gradle is often both executor and project model. That makes it hard to replace
gradually.

Jolt’s answer is not to create another highly configurable build language. It is
to provide a smaller, opinionated, manifest-first layer that can coexist with
Gradle first, explain existing builds, generate Gradle where necessary, and
absorb common build responsibilities over time.

## Target Users

### Early users

Jolt should first serve developers who feel the sharpest pain from JVM/KMP
project complexity while still having relatively disciplined project shapes:

- JVM and Kotlin library authors.
- Kotlin Multiplatform library authors.
- Android and Compose Multiplatform library authors.
- Library authors who publish to Maven Central.
- Developers maintaining JVM/KMP libraries with native dependencies.
- Developers who use Gradle because they must, not because they want to
  hand-author complex build logic.
- Developers who already orchestrate builds outside Gradle with `mise`, shell
  scripts, CI workflows, task runners, or native build systems.
- Developers who want fast, deterministic command-line tools without changing
  their whole build immediately.

### Later users

Jolt may later serve:

- Android application developers.
- Large JVM/KMP monorepos.
- Teams migrating dependency stacks or Java/Kotlin language levels.
- IDE and code review tool authors who want embeddable JVM/KMP source and
  project intelligence.
- Plugin/mod authors who need explicit packaging, shading, relocation, and
  host-provided dependency models.

## Product Principles

### Manifest-first, ecosystem-compatible

Jolt should provide a simple project manifest that describes the project
directly.

It should not ask users to abandon the JVM ecosystem. Maven Central, the JDK,
Kotlin compilers, Android SDKs, native toolchains, Gradle Module Metadata, AARs,
KMP variants, and existing publication systems remain part of the world Jolt
must understand.

### Explain before replacing

Jolt should first make existing JVM, Gradle, Maven, Android, native, and KMP
projects understandable.

Before Jolt replaces a build step, it should be able to explain that step:

- Which dependency version won?
- Why is this artifact present?
- Which target selected this variant?
- Which JDK is used for this task?
- Is this launch using classpath or module path?
- Which native access flags are required?
- Which native artifacts are packaged, linked, loaded, or published?
- Which artifacts are thin, shaded, relocated, minimized, or bundled?
- Which parts still delegate to Gradle?

### Coexist first, absorb later

Jolt should not require a flag-day migration.

The migration path is:

1. Work inside existing Gradle and Maven projects.
2. Explain existing project behavior.
3. Introduce `jolt.toml` as a source of truth.
4. Generate Gradle configuration from `jolt.toml`.
5. Build JVM and Kotlin/JVM projects directly.
6. Build KMP libraries directly.
7. Build Android libraries directly.
8. Consider Android applications later.

Gradle interop is not a compromise. It is the migration bridge.

### Anti-bikeshedding by design

Jolt should expose product-level choices, not endless configuration surfaces.

This applies to formatting, dependency update policy, source-set conventions,
project layout, generated Gradle, packaging defaults, and common build behavior.

Jolt may contain flexible internal machinery, but its user-facing surface should
remain opinionated and small.

### Boring defaults, explicit escape hatches

Common projects should require little configuration.

When a project needs to cross an edge, Jolt should make that explicit:

- Manual dependency cooldown override.
- Explicit Gradle delegation.
- Explicit JVM launch flags.
- Explicit target-specific dependency.
- Explicit native/platform artifact behavior.
- Explicit packaging shape.
- Explicit generated Gradle customization boundary.

Escape hatches should be visible, auditable, and rare.

### One project model, many execution backends

Jolt’s durable core is the workspace model, not any single command.

The same model should power:

- source tools,
- dependency resolution,
- lockfiles,
- toolchain selection,
- JVM launch configuration,
- native/platform artifact modeling,
- packaging and distribution,
- Gradle generation,
- direct builds,
- IDE integration,
- CI diagnostics.

Gradle, Maven, javac, kotlinc, Android tooling, native build systems, and future
direct Jolt execution are backends behind the same project model.

### KMP and Android are first-class, not afterthoughts

Jolt should not pretend KMP is “JVM plus extra targets” or Android is “JVM plus
AAR files.”

The project model must understand:

- source sets,
- targets,
- variants,
- Gradle Module Metadata,
- native klibs,
- Android AAR semantics,
- Android SDK constraints,
- platform-specific artifacts,
- publication layout.

### Native and platform artifacts are first-class

Jolt should not treat native libraries as incidental files copied by custom
build tasks.

For many JVM, KMP, Android, desktop, and library projects, native artifacts are
part of the product. They affect dependency resolution, publication, runtime
loading, launch configuration, Android packaging, Kotlin/Native interop,
platform compatibility, reproducibility, and distribution.

Jolt should model this reality directly.

The goal is not to require Jolt to build all native code itself. The goal is for
Jolt to understand native outputs as project artifacts with platform, target,
runtime, and publication meaning.

### External native builds are valid

Jolt should support projects whose native code is built outside Jolt by tools
such as CMake, Cargo, Zig, Xcode, Android NDK tooling, shell scripts, CI
workflows, or `mise`.

Jolt’s first responsibility is to understand, package, publish, and explain the
resulting artifacts. Owning native compilation can come later, and only where it
meaningfully improves the developer experience.

### Packaging outputs should be explicit products

Jolt should distinguish compiling, packaging, publishing, and running.

A thin library artifact, shaded plugin artifact, runtime image, native
executable, Android AAR, Android app bundle, and KMP publication family are
different products with different dependency and optimization semantics.

Packaging should be explicit because artifact shape affects what consumers
receive, what the runtime loads, what native libraries are included, what names
are relocated, and what metadata is published.

## Product Model

### Workspace model

Jolt’s core product model should describe the project in terms of durable
concepts, not build-script ceremony.

```text
Workspace
  Projects
    Modules
      Source sets
      Targets
      Dependencies
      Native/platform artifacts
      Toolchains
      Publications
      Packages
      Tasks
```

This model should be the foundation for source tools, dependency resolution,
lockfiles, toolchain selection, JVM launch configuration, packaging, Gradle
generation, direct builds, IDEs, and CI.

### Manifest

Jolt should provide a manifest, tentatively `jolt.toml`, that describes the
project the user actually has.

The manifest belongs at the center of the product. It is the user-authored
source of truth for ordinary project structure, dependencies, targets,
toolchains, formatter options, update policy, packaging intent, and publication
intent.

The manifest should be shaped for JVM, KMP, Android, and native-adjacent
libraries rather than copied directly from Node or Python.

### Lockfile

Jolt should have a deterministic lockfile.

The lockfile should support reproducibility and explanation. It should record
selected versions, artifacts, checksums, target/source-set applicability,
variant selection, relevant native/platform artifacts, manual cooldown
overrides, and enough metadata to explain why the selected graph exists.

### Cache

Jolt likely needs a shared cache for artifacts, indexes, toolchains, and
generated/intermediate state.

The cache is not the product, but it supports fast, reproducible operation and
makes the CLI feel modern.

### Execution backends

Jolt should be able to use multiple backends behind one project model:

```text
Jolt project model
  → explain existing Gradle/Maven projects
  → generate Gradle
  → invoke Gradle
  → invoke javac/kotlinc directly
  → consume external native outputs
  → eventually build more targets directly
```

The product should not expose backend complexity unless the user asks Jolt to
explain it.

## Capability Areas

### Source tools

```bash
jolt fmt
jolt fmt --check
jolt lint
jolt lint --fix
```

Source tools provide immediate usefulness in existing projects. They are an
adoption wedge, but not the whole product.

### Dependency tools

```bash
jolt resolve
jolt lock
jolt deps tree
jolt deps why
jolt deps conflicts
jolt deps update
jolt deps add
```

Dependency tools should make Maven, Gradle Module Metadata, KMP variants,
Android artifacts, native/platform artifacts, and lockfiles understandable.

### Native and platform artifact management

Jolt should help projects describe, validate, package, publish, and explain
native and platform-specific artifacts.

This capability is especially important for libraries that expose one logical
dependency to users while internally requiring different artifacts per platform,
architecture, ABI, runtime, or target.

Jolt should make this artifact family understandable as one product rather than
a scattered collection of platform-specific build hacks.

### Packaging and optimization

```bash
jolt package
jolt publish
```

Jolt should model artifact shapes explicitly:

- thin library artifacts,
- fat or uber JARs,
- shaded and relocated JARs,
- minimized or obfuscated artifacts,
- runtime images,
- native-image outputs,
- Android AARs,
- Android APKs or AABs,
- KMP publication families.

Jolt should not need to implement every optimization backend immediately. The
goal is to model packaging intent and make packaging behavior explainable.

### Project explanation

```bash
jolt explain
jolt doctor
```

The explanation layer is core product value.

Jolt should answer:

- Which JDK is this project actually using?
- Which JVM target is Kotlin compiling to?
- Why did this dependency version win?
- Why is this dependency present?
- Which variant was selected for this KMP target?
- Why did Android resolve an AAR instead of a JAR?
- Is this project using classpath or module path?
- Which native access flags are being applied?
- Which native artifact is used for this platform?
- Which artifacts are included in this package?
- Why does the IDE disagree with the command line?
- Which parts of the build still delegate to Gradle?

### Build and execution

```bash
jolt build
jolt test
jolt run
```

Build and execution should grow gradually. Early Jolt can delegate heavily.
Later Jolt can directly execute common project shapes.

### Gradle interop

```bash
jolt gradle generate
jolt gradle sync
```

Gradle interop is a first-class adoption path. Jolt should be able to read
existing projects, explain them, and eventually generate Gradle configuration
from `jolt.toml` so Gradle remains the executor while Jolt owns the authoring
surface.

## Domain Models

### JVM runtime model

Jolt should understand traditional classpath execution and modern module-path
execution.

Classpath mode is the traditional model: a target runs with compiled classes and
JARs supplied as a search path.

Module path is the Java 9+ JPMS model: a target runs with named modules,
explicit dependencies, exported packages, opened packages, and stronger
encapsulation.

Jolt should hide the ceremony but understand the distinction. It should make
launch behavior visible through explanation rather than forcing users to
hand-author every runtime flag.

### JVM native dependency model

For JVM targets, Jolt should recognize that native dependencies involve more
than a Java or Kotlin API.

They may require:

- platform-specific shared libraries,
- runtime loading behavior,
- native access configuration,
- classpath or module-path implications,
- packaging into runnable applications or desktop distributions,
- publication metadata that lets consumers resolve the correct platform
  artifact.

Jolt should help library authors express the intent: this JVM library requires
native runtime artifacts for these platforms.

### KMP model

Jolt must be KMP-first, not merely Java-first.

The shared project model should understand:

- common source sets,
- platform source sets,
- JVM targets,
- Android targets,
- iOS targets,
- macOS/Linux/Windows native targets,
- JS and Wasm targets,
- source-set-specific dependencies,
- target-specific variants,
- KMP publication metadata.

KMP dependencies are not just JARs. A single dependency coordinate may provide
common metadata, JVM artifacts, Android artifacts, native klibs, JS artifacts,
Wasm artifacts, sources, documentation, and Gradle Module Metadata describing
variants.

Jolt’s resolver must understand variant selection as a first-class concept.

### Kotlin/Native dependency model

For Kotlin/Native targets, Jolt should recognize native libraries and headers as
part of interop and linking.

The product goal is to make native interop dependencies visible in the same
project model as KMP source sets, targets, variants, and publications.

Jolt should eventually help library authors publish and consume KMP libraries
whose native components vary by target.

### Wasm native-adjacent model

For Wasm targets, native dependencies should not be assumed to behave like JVM
or Kotlin/Native dependencies.

The long-term model should leave room for separately compiled Wasm modules,
generated bindings, JavaScript interop, browser constraints, host APIs, and
target-specific packaging.

This area should remain deliberately open until ecosystem patterns are clearer.

### Android model

Android should be supported, but not swallowed whole at the start.

An AAR is not just a JAR. It may contain compiled classes, Android resources, an
Android manifest, assets, native libraries, consumer ProGuard/R8 rules, and
Android lint metadata.

Consequences:

- JVM targets consume JARs through classpath or module path.
- Android targets consume AARs through Android-specific packaging behavior.
- Android native libraries must participate in Android artifact construction.
- Android applications require a broader build surface than Android libraries.

Jolt should initially understand Android metadata and delegate actual Android
builds to Gradle/AGP.

### Packaging model

Jolt should model packaging as a separate concern from compilation.

A package may be a thin library artifact, runnable JAR, shaded JAR, relocated
plugin/mod artifact, minimized artifact, runtime image, native executable,
Android AAR, Android app package, or KMP publication family.

Packaging should explain which dependencies are included, excluded, relocated,
merged, minimized, kept, linked, loaded, or provided by the host environment.

### Publishing model

Jolt should model publication to Maven Central and other Maven-compatible
repositories.

Publishing should eventually account for:

- coordinates,
- POM metadata,
- Gradle Module Metadata,
- sources artifacts,
- documentation artifacts,
- signing,
- staging and release flow,
- platform-specific artifact families,
- KMP target-specific publications,
- Android AAR publications,
- native/platform artifact metadata.

### Toolchain model

Jolt should make JVM and KMP toolchains boring.

It should distinguish:

- JDK used to run Gradle, when Gradle is involved,
- JDK used by `javac`,
- JDK used to run tests,
- JDK used by Kotlin/JVM compilation,
- Kotlin JVM bytecode target,
- Android compile SDK,
- Android min SDK,
- Android target SDK,
- native toolchains for KMP targets,
- Wasm/JS target toolchains.

The goal is to eliminate the current fog around Gradle daemon JDK, Java
toolchains, Kotlin target bytecode, IDE SDKs, Android SDKs, and native
toolchains.

## Tool-Level Principles

### Formatting principles

Formatting is layout only.

Jolt should expose a small set of formatter options, such as line width and
indentation. It should not expose named compatibility modes.

The engine may be flexible internally, but the product surface should remain
small.

The formatter may sort imports according to Jolt's documented policy. It must
not rename symbols or perform semantic refactors.

### Import principles

Import ordering is formatting. Import cleanup is a linter action.

Removing unused imports requires more semantic awareness than sorting. Expanding
or collapsing wildcard imports belongs to import cleanup, not pure formatting.
Adding missing imports requires project resolution.

The import lint rule should work conservatively in source-only mode and more
aggressively with project context.

### Lint principles

Lint is not style.

Lint should focus on correctness, maintainability, suspicious code, and project
hazards. Naming conventions are lint diagnostics, not formatter behavior.

Lint should produce diagnostics with optional fixes. Fixes should be classified
as safe, maybe, or unsafe. The default fix mode should apply only safe fixes.

### Dependency principles

Dependency resolution should be explainable.

Cooldown should be simple: one configured duration, manual override when needed.

Jolt should understand Maven POMs, Gradle Module Metadata, KMP variants, Android
AARs, BOMs, platforms, and native/platform artifact families.

Dependency providers should normalize into one internal package model. The
lockfile should record enough information to reproduce and explain selected
artifacts.

### Native artifact principles

Native artifacts should be visible project artifacts, not hidden task outputs.

Jolt should support projects where native code is built externally and ingested
into the JVM/KMP/Android package model.

Jolt should avoid pretending that JNI, FFM, Android native packaging,
Kotlin/Native interop, and Wasm-adjacent native modules are one uniform
mechanism. They share product concerns, but target behavior differs.

### Packaging principles

Thin artifacts should be the default for libraries.

Shading, relocation, minimization, obfuscation, native bundling, and
runtime-image creation should be explicit packaging choices.

Shrinking and obfuscation should be explainable because reachability is limited
by reflection, service loading, serialization, JNI, FFM, annotations, resource
lookup, classpath scanning, and framework conventions.

Package output should be able to explain included, excluded, relocated, merged,
minimized, kept, linked, loaded, and host-provided items.

### Toolchain principles

JDK selection should be explicit and boring.

Jolt should distinguish Gradle daemon JDK, Java toolchain JDK, Kotlin JVM
target, Android SDK levels, native toolchains, and IDE settings.

Toolchain state should be visible through `jolt doctor`. Jolt should own the
translation from manifest intent to compiler/runtime flags.

### JVM launch principles

Classpath mode should be the default for non-modular projects.

Module path should be used when the project opts into JPMS or needs scoped
module behavior.

Native access, `add-opens`, and `add-exports` should be modeled declaratively.

Jolt should generate launch flags consistently for CLI runs, tests, IDEs,
generated Gradle, packaging, and CI.

### Gradle interop principles

Gradle interop is a first-class product feature.

Generated Gradle should be treated as output, not the user-authored source of
truth.

Jolt should preserve compatibility with KGP, AGP, Compose, publishing plugins,
native packaging conventions, and IDE import.

Direct Jolt execution should replace Gradle gradually by capability area.

## Adoption Roadmap

### Phase 1: Companion source tools

Deliver:

- `jolt fmt`
- `jolt lint`
- `jolt fix`

Support Java and Kotlin source files in existing projects.

### Phase 2: Project explanation

Deliver:

- `jolt deps why`
- `jolt deps tree`
- `jolt doctor`
- `jolt explain`

Read existing Gradle, Maven, KMP, Android, and native-adjacent projects.

### Phase 3: Manifest-first Gradle generation

Deliver:

- `jolt.toml`
- `jolt.lock`
- generated Gradle/KGP/AGP configuration,
- generated dependency metadata where appropriate,
- generated packaging configuration where appropriate.

Gradle remains executor. Jolt owns the authoring surface.

### Phase 4: Native artifact explanation and packaging

Native support should begin as an explanation and packaging capability, not as a
full native build system.

Early Jolt should be able to:

- recognize native artifacts associated with JVM, Android, and KMP targets,
- explain which targets require which native artifacts,
- explain how those artifacts are packaged or published,
- preserve native artifact metadata in the lockfile and project model,
- interoperate with Gradle where Gradle still performs actual packaging or build
  execution.

Later Jolt can absorb more direct packaging and build responsibility where the
model is stable.

### Phase 5: Direct JVM and Kotlin/JVM execution

Deliver direct support for:

- Java libraries,
- Kotlin/JVM libraries,
- simple JVM CLI apps,
- unit tests,
- Maven Central publishing.

Jolt invokes JDK and Kotlin compiler tools directly.

### Phase 6: Direct KMP library execution

Deliver direct support for:

- common source sets,
- JVM targets,
- native targets,
- Wasm/JS targets,
- KMP dependency variants,
- KMP publication layout,
- Gradle Module Metadata.

### Phase 7: Android library execution

Deliver direct support for:

- AAR output,
- Android resources,
- manifest handling,
- native libraries,
- consumer rules,
- Android library publishing.

### Phase 8: Android applications

Consider much later.

Android applications require a much broader build surface: variants, flavors,
signing, packaging, resource processing, dexing, R8, bundletool, device
deployment, Compose, IDE expectations, and AGP compatibility.

## Non-Goals

### Permanent non-goals

Jolt should not:

- replace the JDK,
- replace Kotlin compilers,
- replace Maven Central,
- become a general-purpose programmable build language,
- provide arbitrary formatting configuration,
- pretend all JVM, KMP, Android, and native target behavior can be collapsed
  into one generic mechanism.

### Initial non-goals

Jolt should not initially:

- replace Gradle for all projects,
- replace AGP for Android apps,
- replace CMake, Cargo, Zig, Xcode, Android NDK tooling, or other native build
  systems,
- require native code to be built by Jolt,
- reimplement every Gradle plugin,
- support every JVM language equally,
- build complex Android apps directly,
- fully solve Wasm native dependency packaging before ecosystem patterns are
  clearer,
- handle every legacy Maven/Gradle edge case.

Jolt should first make the project visible, explainable, reproducible, and
publishable.

## Long-Term Vision

Jolt eventually becomes the project substrate that Gradle currently is for many
JVM/KMP library projects, but with a smaller, more legible, manifest-first
model.

The long-term command surface:

```bash
jolt fmt
jolt lint
jolt fix

jolt resolve
jolt lock
jolt deps why
jolt deps update

jolt build
jolt test
jolt package
jolt publish
jolt run

jolt doctor
jolt explain
```

The long-term promise:

> Jolt should make JVM and KMP projects easy to describe, explain, lock, update,
> format, build, test, package, publish, and run.

It should preserve the power of the ecosystem while replacing accidental
complexity with visible machinery and boring defaults.

Jolt should make the JVM ecosystem feel less like a haunted Gradle cathedral and
more like a well-lit workshop: sharp tools, labeled drawers, visible machinery,
and boring defaults for decisions that should not consume a team’s attention.
