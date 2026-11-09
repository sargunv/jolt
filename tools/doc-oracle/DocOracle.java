// The JDK's reading of doc comments, for the hunt's doc comment oracle
// (jolt_test_support's `doc_oracle`). It reads requests on stdin and answers
// each with one dump per comment on stdout:
//
//   request:  <count>\n then <count> times <byte length>\n<UTF-8 comment text>
//   response: <count>\n then <count> times <byte length>\n<UTF-8 dump>
//
// A comment is the source text of one `/** … */` comment or of one run of
// `///` lines (joined by \n). The comments of a request are parsed as one
// compilation unit, each before a method of its own, and read with
// `DocTrees.getDocCommentTree`: javac's own `DocCommentParser`, with the
// JDK's Markdown transformer (jdk.internal.md). A dump is javadoc's tree,
// written so that two comments that render the same have the same dump:
//
// - text has its HTML whitespace collapsed, and dropped next to a block-level
//   element and at the ends of a section;
// - text inside <pre> and HTML's raw text elements (<script>, <style>,
//   <textarea>, <title>), the bodies of {@code}, {@literal}, {@snippet} and
//   unknown inline tags, attribute values, {@value} formats, references and
//   HTML comments are written exactly, whitespace at the ends of lines
//   included; a raw text element ends only at its own end tag, as HTML reads
//   it, and holds no elements;
// - an erroneous node is written with its diagnostic code, so a formatted
//   comment can't add or lose a javadoc error;
// - Markdown (a list of trees with Markdown text) is rendered to HTML as the
//   doclet renders it (HtmlDocletWriter.MarkdownHandler: commonmark-java with
//   the tables extension, U+FFFC for each embedded tree), then dumped like
//   HTML, its tags read as HTML reads them (a `>` in a quoted attribute
//   value doesn't end a tag); code is written exactly.
//
// Run by `doc_oracle` with `java --add-modules jdk.internal.md --add-exports
// … DocOracle.java` (see EXPORTS there).

import com.sun.source.doctree.*;
import com.sun.source.tree.ClassTree;
import com.sun.source.tree.CompilationUnitTree;
import com.sun.source.tree.MethodTree;
import com.sun.source.tree.Tree;
import com.sun.source.util.DocTrees;
import com.sun.source.util.JavacTask;
import com.sun.source.util.TreePath;
import java.io.*;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.util.*;
import java.util.concurrent.*;
import javax.tools.*;
import jdk.internal.org.commonmark.ext.gfm.tables.TablesExtension;
import jdk.internal.org.commonmark.node.Node;
import jdk.internal.org.commonmark.parser.Parser;
import jdk.internal.org.commonmark.renderer.html.HtmlRenderer;

public class DocOracle {
    private static final char PLACEHOLDER = '\uFFFC';
    private static final String PLACEHOLDER_BLOCK = "<!--\uFFFC-->";

    private static final Set<String> BLOCK = Set.of(
            "address", "article", "aside", "blockquote", "br", "caption", "center", "col",
            "colgroup", "dd", "details", "dialog", "dir", "div", "dl", "dt", "fieldset",
            "figcaption", "figure", "footer", "form", "h1", "h2", "h3", "h4", "h5", "h6",
            "header", "hgroup", "hr", "li", "main", "menu", "nav", "ol", "p", "pre", "section",
            "summary", "table", "tbody", "td", "tfoot", "th", "thead", "tr", "ul");
    private static final Set<String> EXACT = Set.of("pre", "script", "style", "textarea", "title");
    /** The raw text elements: their content is text up to their own end tag. */
    private static final Set<String> RAW = Set.of("script", "style", "textarea", "title");

    private static final JavaCompiler COMPILER = ToolProvider.getSystemJavaCompiler();
    private static final int TIMEOUT_SECONDS = 10;
    private static ExecutorService executor = newExecutor();
    private static boolean hung;

    private static ExecutorService newExecutor() {
        return Executors.newSingleThreadExecutor(runnable -> {
            var thread = new Thread(runnable);
            thread.setDaemon(true);
            return thread;
        });
    }
    private static final Parser MARKDOWN = Parser.builder()
            .extensions(List.of(TablesExtension.create()))
            .build();
    private static final HtmlRenderer RENDERER = HtmlRenderer.builder()
            .extensions(List.of(TablesExtension.create()))
            .build();

    public static void main(String[] args) throws IOException {
        var in = new BufferedInputStream(System.in, 1 << 16);
        var out = new BufferedOutputStream(System.out, 1 << 16);
        if (args.length > 0) {
            // A comment per file named on the command line, for debugging.
            for (var arg : args) {
                var text = new String(java.nio.file.Files.readAllBytes(java.nio.file.Path.of(arg)),
                        StandardCharsets.UTF_8);
                out.write((dump(List.of(text)).get(0) + "\n").getBytes(StandardCharsets.UTF_8));
            }
            out.flush();
            return;
        }
        while (true) {
            var header = readLine(in);
            if (header == null) {
                break;
            }
            int count = Integer.parseInt(header);
            var comments = new ArrayList<String>(count);
            for (int i = 0; i < count; i++) {
                int length = Integer.parseInt(readLine(in));
                comments.add(new String(in.readNBytes(length), StandardCharsets.UTF_8));
            }
            var dumps = dump(comments);
            out.write((dumps.size() + "\n").getBytes(StandardCharsets.UTF_8));
            for (var dump : dumps) {
                var bytes = dump.getBytes(StandardCharsets.UTF_8);
                out.write((bytes.length + "\n").getBytes(StandardCharsets.UTF_8));
                out.write(bytes);
            }
            out.flush();
            if (hung) {
                System.exit(0);
            }
        }
        out.flush();
    }

    private static String readLine(InputStream in) throws IOException {
        var line = new StringBuilder();
        int c;
        while ((c = in.read()) != -1 && c != '\n') {
            line.append((char) c);
        }
        return c == -1 && line.isEmpty() ? null : line.toString();
    }

    /** The dumps of `comments`, parsed together, or one at a time when that fails. */
    static List<String> dump(List<String> comments) {
        var together = parse(comments);
        if (together != null) {
            return together;
        }
        var dumps = new ArrayList<String>(comments.size());
        for (var comment : comments) {
            var one = parse(List.of(comment));
            dumps.add(one != null ? one.get(0) : "UNPARSABLE");
        }
        return dumps;
    }

    /** The dumps of `comments` read from one compilation unit, or null when it has errors. */
    static List<String> parse(List<String> comments) {
        var source = new StringBuilder("class X {\n");
        for (int i = 0; i < comments.size(); i++) {
            source.append(comments.get(i)).append("\nvoid m").append(i).append("() {}\n");
        }
        source.append("}\n");
        var file = new SimpleJavaFileObject(URI.create("string:///X.java"), JavaFileObject.Kind.SOURCE) {
            @Override
            public CharSequence getCharContent(boolean ignoreEncodingErrors) {
                return source;
            }
        };
        var diagnostics = new DiagnosticCollector<JavaFileObject>();
        var task = (JavacTask) COMPILER.getTask(null, null, diagnostics,
                List.of("-proc:none", "--enable-preview", "--release", "25"), null, List.of(file));
        Iterable<? extends CompilationUnitTree> units;
        try {
            units = task.parse();
        } catch (IOException e) {
            return null;
        }
        for (var diagnostic : diagnostics.getDiagnostics()) {
            if (diagnostic.getKind() == Diagnostic.Kind.ERROR) {
                return null;
            }
        }
        var trees = DocTrees.instance(task);
        var unit = units.iterator().next();
        var type = (ClassTree) unit.getTypeDecls().get(0);
        var classPath = new TreePath(new TreePath(unit), type);
        var dumps = new ArrayList<String>(comments.size());
        for (Tree member : type.getMembers()) {
            if (member instanceof MethodTree method) {
                var path = new TreePath(classPath, method);
                var future = executor.submit(() -> {
                    var tree = trees.getDocCommentTree(path);
                    return tree == null ? "NONE" : new Dumper().comment(tree);
                });
                String dump;
                try {
                    dump = future.get(TIMEOUT_SECONDS, TimeUnit.SECONDS);
                } catch (TimeoutException e) {
                    // javac's reader doesn't finish (its Markdown scanner can
                    // loop at the end of a comment): that is its reading. The
                    // thread can't be stopped, so the JVM ends after this
                    // request, and the caller starts another.
                    dump = "HANG";
                    hung = true;
                    executor = newExecutor();
                } catch (ExecutionException e) {
                    // javac's own reader can fail; the failure is its reading.
                    dump = "CRASH " + e.getCause().getClass().getName();
                } catch (InterruptedException e) {
                    throw new IllegalStateException(e);
                }
                dumps.add(dump);
            }
        }
        return dumps.size() == comments.size() ? dumps : null;
    }

    /** A piece of a dump: text (collapsible), an exact atom, or a block boundary. */
    record Piece(Kind kind, String text) {
        enum Kind { TEXT, ATOM, BLOCK, EXACT_ON, EXACT_OFF }
    }

    static final class Dumper {
        private final StringBuilder out = new StringBuilder();

        String comment(DocCommentTree tree) {
            out.append("FIRST: ");
            section(tree.getFirstSentence());
            out.append("FULL: ");
            section(tree.getFullBody());
            for (var tag : tree.getBlockTags()) {
                blockTag(tag);
            }
            return out.toString();
        }

        private void section(List<? extends DocTree> trees) {
            var pieces = new ArrayList<Piece>();
            list(trees, pieces);
            out.append(normalize(pieces)).append('\n');
        }

        private void blockTag(DocTree tag) {
            var head = new StringBuilder("TAG ");
            List<? extends DocTree> description;
            switch (tag) {
                case ParamTree t -> {
                    head.append("@param ").append(t.isTypeParameter() ? "<" : "")
                            .append(t.getName()).append(t.isTypeParameter() ? ">" : "");
                    description = t.getDescription();
                }
                case ThrowsTree t -> {
                    head.append('@').append(t.getTagName()).append(' ')
                            .append(collapse(t.getExceptionName().getSignature()));
                    description = t.getDescription();
                }
                case ProvidesTree t -> {
                    head.append("@provides ").append(collapse(t.getServiceType().getSignature()));
                    description = t.getDescription();
                }
                case UsesTree t -> {
                    head.append("@uses ").append(collapse(t.getServiceType().getSignature()));
                    description = t.getDescription();
                }
                case SerialFieldTree t -> {
                    head.append("@serialField ").append(t.getName()).append(' ')
                            .append(collapse(t.getType().getSignature()));
                    description = t.getDescription();
                }
                case SpecTree t -> {
                    head.append("@spec ").append(escape(t.getURL().getBody()));
                    description = t.getTitle();
                }
                case SeeTree t -> {
                    head.append("@see");
                    description = t.getReference();
                }
                case ErroneousTree t -> {
                    head.append("ERRONEOUS ").append(t.getDiagnostic().getCode()).append(' ')
                            .append(collapse(t.getBody()));
                    description = List.of();
                }
                case BlockTagTree t -> {
                    head.append('@').append(t.getTagName());
                    description = switch (t) {
                        case AuthorTree a -> a.getName();
                        case DeprecatedTree d -> d.getBody();
                        case HiddenTree h -> h.getBody();
                        case ReturnTree r -> r.getDescription();
                        case SerialTree s -> s.getDescription();
                        case SerialDataTree s -> s.getDescription();
                        case SinceTree s -> s.getBody();
                        case VersionTree v -> v.getBody();
                        case UnknownBlockTagTree u -> u.getContent();
                        default -> throw new IllegalStateException(t.getKind().toString());
                    };
                }
                default -> throw new IllegalStateException(tag.getKind().toString());
            }
            out.append(head).append(": ");
            section(description);
        }

        /** Appends the pieces of `trees`, rendering Markdown as the doclet does. */
        private void list(List<? extends DocTree> trees, List<Piece> pieces) {
            if (trees.stream().noneMatch(t -> t.getKind() == DocTree.Kind.MARKDOWN)) {
                for (var tree : trees) {
                    node(tree, pieces);
                }
                return;
            }
            var input = new StringBuilder();
            var embedded = new ArrayList<List<Piece>>();
            for (var tree : trees) {
                if (tree instanceof RawTextTree raw) {
                    var code = raw.getContent();
                    int start = 0;
                    int pos;
                    while ((pos = code.indexOf(PLACEHOLDER, start)) != -1) {
                        input.append(code, start, pos).append(PLACEHOLDER);
                        embedded.add(List.of(new Piece(Piece.Kind.TEXT, String.valueOf(PLACEHOLDER))));
                        start = pos + 1;
                    }
                    input.append(code.substring(start));
                } else {
                    var inner = new ArrayList<Piece>();
                    node(tree, inner);
                    embedded.add(inner);
                    if (tree.getKind() == DocTree.Kind.SNIPPET) {
                        if (!input.isEmpty() && input.charAt(input.length() - 1) != '\n') {
                            input.append('\n');
                        }
                        input.append(PLACEHOLDER_BLOCK);
                    } else {
                        input.append(PLACEHOLDER);
                    }
                }
            }
            Node document = MARKDOWN.parse(input.toString());
            html(RENDERER.render(document), embedded.iterator(), pieces);
        }

        /** The pieces of rendered HTML, with the embedded trees for the placeholders. */
        private static void html(String html, Iterator<List<Piece>> embedded, List<Piece> pieces) {
            int i = 0;
            // The raw text element that is open: its content is text up to its end tag.
            String raw = null;
            while (i < html.length()) {
                if (raw == null && html.startsWith(PLACEHOLDER_BLOCK, i)) {
                    pieces.add(new Piece(Piece.Kind.BLOCK, ""));
                    pieces.addAll(embedded.next());
                    pieces.add(new Piece(Piece.Kind.BLOCK, ""));
                    i += PLACEHOLDER_BLOCK.length();
                    continue;
                }
                char c = html.charAt(i);
                if (c == PLACEHOLDER) {
                    pieces.addAll(embedded.next());
                    i++;
                } else if (c == '<' && (raw == null ? startsTag(html, i) : endsRaw(html, i, raw))) {
                    int end = tagEnd(html, i);
                    var tag = new StringBuilder();
                    for (int k = i; k < end; k++) {
                        // An embedded tree in an attribute value.
                        if (html.charAt(k) == PLACEHOLDER) {
                            for (var piece : embedded.next()) {
                                tag.append(piece.text());
                            }
                        } else {
                            tag.append(html.charAt(k));
                        }
                    }
                    var name = tagName(tag.toString());
                    boolean closing = tag.charAt(1) == '/';
                    element(name, closing, tag.toString(), pieces);
                    if (name.equals("code") && !closing) {
                        pieces.add(new Piece(Piece.Kind.EXACT_ON, name));
                    } else if (name.equals("code")) {
                        pieces.add(pieces.size() - 1, new Piece(Piece.Kind.EXACT_OFF, name));
                    }
                    if (RAW.contains(name)) {
                        raw = closing ? null : name;
                    }
                    i = end;
                } else {
                    int next = i + 1;
                    while (next < html.length() && html.charAt(next) != '<'
                            && html.charAt(next) != PLACEHOLDER) {
                        next++;
                    }
                    pieces.add(new Piece(Piece.Kind.TEXT, html.substring(i, next)));
                    i = next;
                }
            }
        }

        /** Whether a tag, comment or declaration starts at `i` (a `<`). */
        private static boolean startsTag(String html, int i) {
            if (i + 1 >= html.length()) {
                return false;
            }
            char next = html.charAt(i + 1);
            if (next == '/') {
                return i + 2 < html.length() && Character.isLetter(html.charAt(i + 2));
            }
            return Character.isLetter(next) || next == '!' || next == '?';
        }

        /** Whether the end tag of the raw text element `name` starts at `i`. */
        private static boolean endsRaw(String html, int i, String name) {
            int after = i + 2 + name.length();
            return html.startsWith("</", i) && after < html.length()
                    && html.regionMatches(true, i + 2, name, 0, name.length())
                    && " \t\n\f\r/>".indexOf(html.charAt(after)) >= 0;
        }

        /**
         * The end of the tag at `i`: after its `>`, outside quoted attribute
         * values (after `-->` for a comment), or the end of the text.
         */
        private static int tagEnd(String html, int i) {
            if (html.startsWith("<!--", i)) {
                int end = html.indexOf("-->", i + 4);
                return end < 0 ? html.length() : end + 3;
            }
            boolean eq = false;
            for (int k = i + 1; k < html.length(); k++) {
                char c = html.charAt(k);
                if (c == '>') {
                    return k + 1;
                } else if ((c == '"' || c == '\'') && eq) {
                    int close = html.indexOf(c, k + 1);
                    if (close < 0) {
                        return html.length();
                    }
                    k = close;
                    eq = false;
                } else if (c == '=') {
                    eq = true;
                } else if (" \t\n\f\r".indexOf(c) < 0) {
                    eq = false;
                }
            }
            return html.length();
        }

        private static String tagName(String tag) {
            int start = tag.startsWith("</") ? 2 : 1;
            int end = start;
            while (end < tag.length() && (Character.isLetterOrDigit(tag.charAt(end)) || tag.charAt(end) == '-')) {
                end++;
            }
            return tag.substring(start, end).toLowerCase(Locale.ROOT);
        }

        /** An HTML element's tag: an atom, a block boundary around it if it is block-level. */
        private static void element(String name, boolean closing, String text, List<Piece> pieces) {
            boolean block = BLOCK.contains(name);
            if (block) {
                pieces.add(new Piece(Piece.Kind.BLOCK, ""));
            }
            if (EXACT.contains(name) && closing) {
                pieces.add(new Piece(Piece.Kind.EXACT_OFF, name));
            }
            pieces.add(new Piece(Piece.Kind.ATOM, text));
            if (EXACT.contains(name) && !closing) {
                pieces.add(new Piece(Piece.Kind.EXACT_ON, name));
            }
            if (block) {
                pieces.add(new Piece(Piece.Kind.BLOCK, ""));
            }
        }

        private void node(DocTree tree, List<Piece> pieces) {
            switch (tree) {
                case EscapeTree t -> pieces.add(atom("escape:" + t.getBody()));
                case ErroneousTree t -> pieces.add(atom("ERRONEOUS " + t.getDiagnostic().getCode() + " "
                        + collapse(t.getBody())));
                case TextTree t -> pieces.add(t.getBody().startsWith("<![CDATA[")
                        ? new Piece(Piece.Kind.ATOM, escape(t.getBody()))
                        : new Piece(Piece.Kind.TEXT, t.getBody()));
                case EntityTree t -> pieces.add(atom("&" + t.getName() + ";"));
                case StartElementTree t -> {
                    var tag = new StringBuilder("<").append(t.getName().toString().toLowerCase(Locale.ROOT));
                    for (var attribute : t.getAttributes()) {
                        tag.append(' ').append(attribute((AttributeTree) attribute));
                    }
                    tag.append(t.isSelfClosing() ? "/>" : ">");
                    element(t.getName().toString().toLowerCase(Locale.ROOT), false, tag.toString(), pieces);
                }
                case EndElementTree t -> {
                    var name = t.getName().toString().toLowerCase(Locale.ROOT);
                    element(name, true, "</" + name + ">", pieces);
                }
                case CommentTree t -> pieces.add(atom("comment:" + escape(t.getBody())));
                case DocTypeTree t -> pieces.add(atom("doctype:" + escape(t.getText())));
                case LiteralTree t -> pieces.add(atom("{@" + t.getKind().tagName + " "
                        + escape(t.getBody().getBody()) + "}"));
                case LinkTree t -> {
                    pieces.add(atom("{@" + t.getTagName() + " " + reference(t.getReference())));
                    list(t.getLabel(), pieces);
                    pieces.add(atom("}"));
                }
                case ValueTree t -> pieces.add(atom("{@value "
                        + (t.getFormat() == null ? "" : escape(t.getFormat().getBody()) + " ")
                        + reference(t.getReference()) + "}"));
                case SnippetTree t -> {
                    var attributes = new StringBuilder();
                    for (var attribute : t.getAttributes()) {
                        attributes.append(' ').append(attribute((AttributeTree) attribute));
                    }
                    pieces.add(atom("{@snippet" + attributes
                            + (t.getBody() == null ? "" : ":" + escape(t.getBody().getBody())) + "}"));
                }
                case DocRootTree t -> pieces.add(atom("{@docRoot}"));
                case InheritDocTree t -> pieces.add(atom("{@inheritDoc "
                        + (t.getSupertype() == null ? "" : reference(t.getSupertype())) + "}"));
                case SystemPropertyTree t -> pieces.add(atom("{@systemProperty " + t.getPropertyName() + "}"));
                case IndexTree t -> {
                    pieces.add(atom("{@index " + escape(((TextTree) t.getSearchTerm()).getBody())));
                    list(t.getDescription(), pieces);
                    pieces.add(atom("}"));
                }
                case ReturnTree t -> {
                    pieces.add(atom("{@return"));
                    list(t.getDescription(), pieces);
                    pieces.add(atom("}"));
                }
                case SummaryTree t -> {
                    pieces.add(atom("{@summary"));
                    list(t.getSummary(), pieces);
                    pieces.add(atom("}"));
                }
                case UnknownInlineTagTree t -> {
                    var content = new StringBuilder();
                    for (var part : t.getContent()) {
                        content.append(part instanceof TextTree text ? text.getBody() : part.toString());
                    }
                    pieces.add(atom("{@" + t.getTagName() + " " + escape(content.toString()) + "}"));
                }
                case ReferenceTree t -> pieces.add(atom(reference(t)));
                case RawTextTree t -> pieces.add(new Piece(Piece.Kind.TEXT, t.getContent()));
                default -> pieces.add(atom("?" + tree.getKind() + ":" + escape(tree.toString())));
            }
        }

        private static Piece atom(String text) {
            return new Piece(Piece.Kind.ATOM, text);
        }

        private static String reference(ReferenceTree tree) {
            // Whitespace inside a reference doesn't change what it refers to.
            return tree == null ? "ref:-" : "ref:" + collapse(tree.getSignature());
        }

        private String attribute(AttributeTree attribute) {
            var text = new StringBuilder(attribute.getName().toString().toLowerCase(Locale.ROOT));
            if (attribute.getValueKind() != AttributeTree.ValueKind.EMPTY) {
                text.append('=').append(switch (attribute.getValueKind()) {
                    case UNQUOTED -> "";
                    case SINGLE -> "'";
                    case DOUBLE -> "\"";
                    default -> "";
                });
                for (var part : attribute.getValue()) {
                    var inner = new ArrayList<Piece>();
                    node(part, inner);
                    for (var piece : inner) {
                        text.append(piece.kind() == Piece.Kind.TEXT ? escape(piece.text()) : piece.text());
                    }
                }
                text.append(switch (attribute.getValueKind()) {
                    case SINGLE -> "'";
                    case DOUBLE -> "\"";
                    default -> "";
                });
            }
            return text.toString();
        }
    }

    /**
     * The pieces as one line: text collapsed (exactly as written inside an exact
     * zone), dropped next to a block boundary and at the ends.
     */
    static String normalize(List<Piece> pieces) {
        var out = new ArrayList<String>();
        // The open exact elements, innermost last.
        var exactNames = new ArrayDeque<String>();
        var text = new StringBuilder();
        boolean pendingSpace = false;
        boolean atBoundary = true;
        for (var piece : pieces) {
            switch (piece.kind()) {
                case EXACT_ON -> {
                    // A raw text element holds no elements.
                    if (!exactNames.isEmpty() && RAW.contains(exactNames.peekLast())) {
                        continue;
                    }
                    if (!exactNames.isEmpty()) {
                        if (!text.isEmpty()) {
                            out.add(exact(text.toString()));
                        }
                    } else {
                        flushText(out, text, pendingSpace && !atBoundary);
                    }
                    text.setLength(0);
                    pendingSpace = false;
                    exactNames.addLast(piece.text());
                }
                case EXACT_OFF -> {
                    // Only its own end tag ends a raw text element, and an end
                    // tag closes the elements opened after its element.
                    var top = exactNames.peekLast();
                    if (top == null || (RAW.contains(top) && !top.equals(piece.text()))
                            || !exactNames.contains(piece.text())) {
                        continue;
                    }
                    if (!text.isEmpty()) {
                        out.add(exact(text.toString()));
                    }
                    text.setLength(0);
                    while (!exactNames.removeLast().equals(piece.text())) {
                        // The elements opened inside it.
                    }
                }
                case TEXT -> {
                    if (!exactNames.isEmpty()) {
                        text.append(piece.text());
                        continue;
                    }
                    var body = piece.text();
                    for (int i = 0; i < body.length(); i++) {
                        char c = body.charAt(i);
                        if (c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\f') {
                            if (!text.isEmpty() || !atBoundary) {
                                pendingSpace = true;
                            }
                        } else {
                            if (pendingSpace && (!text.isEmpty() || !atBoundary)) {
                                text.append(' ');
                            }
                            pendingSpace = false;
                            text.append(c);
                        }
                    }
                    if (!text.isEmpty()) {
                        atBoundary = false;
                    }
                }
                case ATOM -> {
                    if (!exactNames.isEmpty()) {
                        if (!text.isEmpty()) {
                            out.add(exact(text.toString()));
                        }
                        text.setLength(0);
                        out.add("@" + piece.text());
                        continue;
                    }
                    flushText(out, text, pendingSpace && !atBoundary);
                    text.setLength(0);
                    pendingSpace = false;
                    out.add("@" + piece.text());
                    atBoundary = false;
                }
                case BLOCK -> {
                    if (!exactNames.isEmpty()) {
                        continue;
                    }
                    flushText(out, text, false);
                    text.setLength(0);
                    pendingSpace = false;
                    atBoundary = true;
                }
            }
        }
        if (!exactNames.isEmpty()) {
            if (!text.isEmpty()) {
                out.add(exact(text.toString()));
            }
        } else {
            flushText(out, text, false);
        }
        return String.join(" | ", out);
    }

    /** Adds `text` (and the space after it, when `space`) as a text piece. */
    private static void flushText(List<String> out, StringBuilder text, boolean space) {
        if (!text.isEmpty() || space) {
            out.add("\"" + text + (space ? " " : "") + "\"");
        }
    }

    /** Exact text: as written, whitespace at the ends of lines included. */
    static String exact(String text) {
        return "=" + escape(text);
    }

    static String collapse(String text) {
        return text.replaceAll("[ \\t\\n\\r\\f]+", " ").strip();
    }

    static String escape(String text) {
        return text.replace("\\", "\\\\").replace("\n", "\\n").replace("\r", "\\r").replace("\t", "\\t");
    }
}
