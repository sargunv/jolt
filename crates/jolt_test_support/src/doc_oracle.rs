//! The doc comment oracle: what the JDK reads in each doc comment, before and
//! after formatting.
//!
//! The comments are read by `tools/doc-oracle/DocOracle.java`, which runs
//! javac's own doc comment parser (`DocTrees.getDocCommentTree`) and renders
//! Markdown as the doclet does, and dumps each comment so that two comments
//! javadoc renders the same have the same dump (see that file). The JVM
//! starts once per batch: the hunt records the comments of every variant
//! whose doc comments changed while it formats (`record`), and reads them all
//! at the end (`resolve`), in a few JVMs, each comment once.
//!
//! A doc comment is a `/** … */` comment, or a run of `///` lines grouped as
//! javac groups them: lines that start with `///` (`////` too), one after the
//! other with only spaces and tabs before each and no blank line between;
//! the first may follow code on its line. Each is compared in a canonical
//! text that javac reads the same: a line of a `/**` comment loses the
//! whitespace before its leading `*`, and a `///` line the whitespace before
//! its `///`. Comments whose canonical texts are equal need no JVM.
//!
//! Enabled only by the hunt and `jcheck` ([`enable`]); `mise run test` never
//! starts a JVM.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Read, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

/// The comments of one JVM request.
const BATCH: usize = 256;

static ENABLED: AtomicBool = AtomicBool::new(false);
static STATE: OnceLock<Mutex<State>> = OnceLock::new();

thread_local! {
    /// The file and the variant the current thread checks.
    static CONTEXT: std::cell::RefCell<(PathBuf, String)> =
        const { std::cell::RefCell::new((PathBuf::new(), String::new())) };
}

/// Turns the oracle on, with the repository root (for the tool's source).
pub fn enable(root: &Path) {
    ENABLED.store(true, Ordering::SeqCst);
    let state = STATE.get_or_init(|| Mutex::new(State::default()));
    state.lock().expect("doc oracle state").root = root.to_path_buf();
}

pub(crate) fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Names the file whose variants the current thread checks next.
pub fn set_file(path: &Path) {
    CONTEXT.with(|context| context.borrow_mut().0 = path.to_path_buf());
}

/// Names the variant the current thread checks next.
pub(crate) fn set_variant(label: &str) {
    CONTEXT.with(|context| label.clone_into(&mut context.borrow_mut().1));
}

#[derive(Default)]
struct State {
    root: PathBuf,
    ids: HashMap<String, u32>,
    texts: Vec<String>,
    dumps: Vec<Option<String>>,
    /// The comparisons to make: the sorted comment ids of a source and of
    /// its formatted output, with the first variant that had them.
    pending: HashMap<(Vec<u32>, Vec<u32>), (PathBuf, String)>,
}

impl State {
    fn intern(&mut self, text: String) -> u32 {
        if let Some(&id) = self.ids.get(&text) {
            return id;
        }
        let id = u32::try_from(self.texts.len()).expect("comment count fits in u32");
        self.ids.insert(text.clone(), id);
        self.texts.push(text);
        self.dumps.push(None);
        id
    }
}

/// Records the doc comments of a source and its formatted output (canonical
/// texts, see the module docs) for the current variant, when they differ.
pub(crate) fn record(source: &[String], output: &[String]) {
    let mut source = source.to_vec();
    let mut output = output.to_vec();
    source.sort_unstable();
    output.sort_unstable();
    if source == output {
        return;
    }
    let (path, label) = CONTEXT.with(|context| context.borrow().clone());
    let mut state = STATE
        .get()
        .expect("doc oracle enabled")
        .lock()
        .expect("doc oracle state");
    let mut ids = |texts: Vec<String>| {
        let mut ids: Vec<u32> = texts.into_iter().map(|text| state.intern(text)).collect();
        ids.sort_unstable();
        ids
    };
    let key = (ids(source), ids(output));
    state.pending.entry(key).or_insert((path, label));
}

/// One comparison that failed: the file and variant, the check, and the
/// comments that javadoc reads differently.
pub struct DocFailure {
    pub path: PathBuf,
    pub label: String,
    pub check: &'static str,
    pub detail: String,
}

/// What [`resolve`] did: the comparisons it made, the comments the JDK read
/// for them, and the comparisons whose readings differ.
pub struct Resolved {
    pub comparisons: usize,
    pub comments: usize,
    pub failures: Vec<DocFailure>,
}

/// Reads every recorded comment with the JDK and returns the comparisons
/// whose readings differ. Clears the recorded comparisons.
///
/// # Panics
///
/// When the JDK can't be started (the hunt needs the `java` that
/// `mise.toml` pins).
#[must_use]
pub fn resolve() -> Resolved {
    let Some(state) = STATE.get() else {
        return Resolved {
            comparisons: 0,
            comments: 0,
            failures: Vec::new(),
        };
    };
    let mut state = state.lock().expect("doc oracle state");
    let pending = std::mem::take(&mut state.pending);
    let mut needed: Vec<u32> = pending
        .keys()
        .flat_map(|(source, output)| source.iter().chain(output))
        .copied()
        .filter(|&id| state.dumps[id as usize].is_none())
        .collect();
    needed.sort_unstable();
    needed.dedup();
    if !needed.is_empty() {
        let texts: Vec<&str> = needed
            .iter()
            .map(|&id| state.texts[id as usize].as_str())
            .collect();
        let dumps = dump_all(&state.root, &texts);
        for (&id, dump) in needed.iter().zip(dumps) {
            state.dumps[id as usize] = Some(dump);
        }
    }
    let mut failures = Vec::new();
    let comparisons = pending.len();
    let comments = needed.len();
    let mut pending: Vec<_> = pending.into_iter().collect();
    pending.sort_by(|left, right| left.1.cmp(&right.1));
    for ((source, output), (path, label)) in pending {
        let dump = |id: &u32| state.dumps[*id as usize].clone().unwrap_or_default();
        let mut before: Vec<(String, u32)> = source.iter().map(|id| (dump(id), *id)).collect();
        let mut after: Vec<(String, u32)> = output.iter().map(|id| (dump(id), *id)).collect();
        before.sort();
        after.sort();
        let same = before.len() == after.len()
            && before
                .iter()
                .zip(&after)
                .all(|(left, right)| left.0 == right.0);
        if same {
            continue;
        }
        let lost = difference(&before, &after);
        let gained = difference(&after, &before);
        let errors = |side: &[&(String, u32)]| {
            side.iter()
                .map(|(dump, _)| dump.matches("ERRONEOUS").count())
                .sum::<usize>()
        };
        let check = if errors(&gained) > errors(&lost) {
            "javadoc-error"
        } else {
            "javadoc-changed"
        };
        let mut detail = String::new();
        for (name, side) in [("before", &lost), ("after", &gained)] {
            for (dump, id) in side {
                let _ = write!(
                    detail,
                    "--- {name}:\n{}\n--- {name} (javadoc):\n{dump}\n",
                    state.texts[*id as usize]
                );
            }
        }
        failures.push(DocFailure {
            path,
            label,
            check,
            detail,
        });
    }
    Resolved {
        comparisons,
        comments,
        failures,
    }
}

/// The entries of `left` (sorted by dump) whose dumps `right` lacks.
fn difference<'v>(left: &'v [(String, u32)], right: &[(String, u32)]) -> Vec<&'v (String, u32)> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for (dump, _) in right {
        *counts.entry(dump.as_str()).or_default() += 1;
    }
    left.iter()
        .filter(|(dump, _)| {
            let count = counts.entry(dump.as_str()).or_default();
            if *count > 0 {
                *count -= 1;
                false
            } else {
                true
            }
        })
        .collect()
}

/// The modules and packages the tool reads: javac's Markdown transformer
/// and the commonmark-java that the doclet renders Markdown with.
const EXPORTS: &[&str] = &[
    "jdk.internal.md/jdk.internal.org.commonmark.parser",
    "jdk.internal.md/jdk.internal.org.commonmark.node",
    "jdk.internal.md/jdk.internal.org.commonmark.renderer.html",
    "jdk.internal.md/jdk.internal.org.commonmark.ext.gfm.tables",
];

/// One JVM running the tool.
struct Jvm {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Jvm {
    fn start(root: &Path) -> Self {
        let mut command = Command::new("java");
        command.arg("--add-modules").arg("jdk.internal.md");
        for export in EXPORTS {
            command
                .arg("--add-exports")
                .arg(format!("{export}=ALL-UNNAMED"));
        }
        command
            .arg(root.join("tools/doc-oracle/DocOracle.java"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        let mut child = command.spawn().unwrap_or_else(|error| {
            panic!("the doc oracle needs the JDK that mise.toml pins (`java`): {error}")
        });
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = BufReader::new(child.stdout.take().expect("piped stdout"));
        Self {
            child,
            stdin,
            stdout,
        }
    }

    /// The dumps of `comments`, or `None` when the JVM has stopped (it
    /// stops after a request in which javac's reader didn't finish).
    fn dump(&mut self, comments: &[&str]) -> Option<Vec<String>> {
        let mut request = format!("{}\n", comments.len()).into_bytes();
        for comment in comments {
            request.extend_from_slice(format!("{}\n", comment.len()).as_bytes());
            request.extend_from_slice(comment.as_bytes());
        }
        self.stdin.write_all(&request).ok()?;
        self.stdin.flush().ok()?;
        let count = self.read_number()?;
        assert_eq!(count, comments.len(), "doc oracle answer count");
        (0..count)
            .map(|_| {
                let length = self.read_number()?;
                let mut bytes = vec![0; length];
                self.stdout.read_exact(&mut bytes).ok()?;
                Some(String::from_utf8(bytes).expect("UTF-8 from the doc oracle"))
            })
            .collect()
    }

    fn read_number(&mut self) -> Option<usize> {
        let mut line = String::new();
        self.stdout.read_line(&mut line).ok()?;
        line.trim().parse().ok()
    }
}

impl Drop for Jvm {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The dumps of `texts`, read in batches by a few JVMs at once.
fn dump_all(root: &Path, texts: &[&str]) -> Vec<String> {
    let batches: Vec<&[&str]> = texts.chunks(BATCH).collect();
    let workers = std::thread::available_parallelism()
        .map_or(4, std::num::NonZero::get)
        .min(16)
        .min(batches.len());
    let next = Mutex::new(0usize);
    let results: Mutex<Vec<Option<Vec<String>>>> = Mutex::new(vec![None; batches.len()]);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                let mut jvm = Jvm::start(root);
                loop {
                    let index = {
                        let mut next = next.lock().expect("batch queue");
                        let index = *next;
                        *next += 1;
                        index
                    };
                    let Some(batch) = batches.get(index) else {
                        break;
                    };
                    let dumps = jvm.dump(batch).unwrap_or_else(|| {
                        jvm = Jvm::start(root);
                        jvm.dump(batch).expect("the doc oracle answers a request")
                    });
                    results.lock().expect("batch results")[index] = Some(dumps);
                }
            });
        }
    });
    results
        .into_inner()
        .expect("batch results")
        .into_iter()
        .flat_map(|dumps| dumps.expect("every batch answered"))
        .collect()
}
