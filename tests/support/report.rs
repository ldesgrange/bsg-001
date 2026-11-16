//! Renders the oaths as a static site that shows the results of the last run.
//!
//! An example is green when it passed, red when it failed, and grey when it has no current result.
//! The sentences of an example are coloured by its kind: e2e when one of its steps runs the command, core otherwise.

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;
use std::ops::Range;
use std::path::Path;

use pulldown_cmark::{CowStr, Event, Options, Parser, Tag, TagEnd, html};
use varar_core::hash::hash_source;
use varar_core::offsets::byte_index;
use varar_core::reference::OathWorkspace;
use varar_core::registry::{Registry, StepRegistration};
use varar_core::span::Span;
use varar_runner::plan_oath;

/// The outcome of an example, ordered so that the worst outcome of several is the greatest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    Passed,
    NotRun,
    Failed,
}

impl Status {
    fn class(self) -> &'static str {
        match self {
            Status::Passed => "passed",
            Status::NotRun => "not-run",
            Status::Failed => "failed",
        }
    }
}

/// Whether an example runs the command (e2e) or checks the code directly (core).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Core,
    E2e,
}

impl Kind {
    fn class(self) -> &'static str {
        match self {
            Kind::Core => "core",
            Kind::E2e => "e2e",
        }
    }
}

/// What a range of an oath is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mark {
    /// The whole of an example.
    Example(Status),
    /// A sentence that matched a step.
    Step(Kind, Status),
    /// Where a failure points.
    Anchor(String),
    /// A table cell or parameter whose actual value differs from the expected one.
    Cell(String),
}

/// A mark over a byte range of an oath.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Marked {
    pub range: Range<usize>,
    pub mark: Mark,
}

#[derive(Default)]
struct Counts {
    passed: usize,
    failed: usize,
    not_run: usize,
    e2e: usize,
    core: usize,
}

impl Counts {
    fn add(&mut self, status: Status, kind: Kind) {
        match status {
            Status::Passed => self.passed += 1,
            Status::NotRun => self.not_run += 1,
            Status::Failed => self.failed += 1,
        }
        match kind {
            Kind::E2e => self.e2e += 1,
            Kind::Core => self.core += 1,
        }
    }

    fn total(&self) -> usize {
        self.passed + self.failed + self.not_run
    }

    fn summary(&self) -> String {
        format!(
            "<p class=\"summary\"><span class=\"badge passed\">{} passed</span> <span class=\"badge failed\">{} failed</span> <span class=\"badge not-run\">{} not run</span> <span class=\"badge e2e\">{} e2e</span> <span class=\"badge core\">{} core</span></p>\n",
            self.passed, self.failed, self.not_run, self.e2e, self.core
        )
    }
}

struct Failure {
    message: String,
    anchor: Option<(usize, usize)>,
    cells: Vec<(usize, usize, String)>,
    doc_path: Option<String>,
}

struct Outcome {
    status: Status,
    failure: Option<Failure>,
}

struct Results {
    source_hash: String,
    /// The hash of each other document whose steps the oath used, when the oath was run.
    documents: HashMap<String, String>,
    /// Each outcome, by the name and lines varar records for its example.
    examples: HashMap<(String, Vec<usize>), Outcome>,
}

/// The results varar wrote for an oath in `.varar/`, when there are any.
fn read_results(root: &Path, oath_path: &str) -> Option<Results> {
    let text = std::fs::read_to_string(varar_runner::results::result_file_path(root, oath_path)).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    let string = |value: &serde_json::Value, key: &str| value.get(key).and_then(serde_json::Value::as_str).map(str::to_owned);
    let offset = |value: &serde_json::Value, key: &str| value.get(key).and_then(serde_json::Value::as_u64).map(|n| n as usize);
    let documents = json["documents"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|document| Some((string(document, "path")?, string(document, "sourceHash")?)))
        .collect();
    let examples = json["examples"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|example| {
            let status = match example["status"].as_str()? {
                "passed" => Status::Passed,
                "failed" => Status::Failed,
                _ => Status::NotRun,
            };
            let failure = example.get("failure").map(|failure| Failure {
                message: string(failure, "message").unwrap_or_default(),
                anchor: failure.get("anchor").and_then(|anchor| Some((offset(anchor, "from")?, offset(anchor, "to")?))),
                cells: failure["cells"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|cell| Some((offset(cell, "from")?, offset(cell, "to")?, unquoted(string(cell, "actual")?))))
                    .collect(),
                doc_path: string(failure, "docPath"),
            });
            let lines = example["lines"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|line| line.as_u64().map(|n| n as usize))
                .collect();
            Some(((string(example, "name")?, lines), Outcome { status, failure }))
        })
        .collect();
    Some(Results {
        source_hash: string(&json, "sourceHash")?,
        documents,
        examples,
    })
}

/// An actual value as it would be written in the oath: a doc string is recorded quoted, with its line breaks escaped.
fn unquoted(actual: String) -> String {
    serde_json::from_str(&actual).unwrap_or(actual)
}

/// The page an oath becomes: `specs/run.md` becomes `run.html`.
fn page_name(oath_path: &str) -> String {
    let stem = Path::new(oath_path)
        .file_stem()
        .map_or_else(|| oath_path.to_owned(), |stem| stem.to_string_lossy().into_owned());
    format!("{stem}.html")
}

/// Render every oath, with the results varar last wrote under `root`, to a site in `out`.
pub fn render_site(
    root: &Path,
    out: &Path,
    oaths: &[(String, String)],
    registry: &Registry,
    workspace: &OathWorkspace,
    is_e2e: impl Fn(&StepRegistration) -> bool,
) -> std::io::Result<()> {
    let sources: HashMap<&str, &str> = oaths.iter().map(|(path, source)| (path.as_str(), source.as_str())).collect();
    let mut marks: HashMap<String, Vec<Marked>> = HashMap::new();
    let mut push = |doc: &str, from: usize, to: usize, mark: Mark| {
        if let Some(source) = sources.get(doc) {
            let range = byte_index(source, from)..byte_index(source, to);
            marks.entry(doc.to_owned()).or_default().push(Marked { range, mark });
        }
    };
    let span = |span: &Span| (span.start_offset, span.end_offset);
    let mut pages = Vec::new();
    for (path, source) in oaths {
        let plan = plan_oath(path, source, registry, workspace);
        let results = read_results(root, path);
        let current = |doc: &str, hash: &str| sources.get(doc).is_some_and(|source| hash_source(source) == hash);
        let fresh = results
            .as_ref()
            .filter(|results| current(path, &results.source_hash) && results.documents.iter().all(|(doc, hash)| current(doc, hash)));
        let banner = match (&results, fresh) {
            (None, _) => Some("These examples have not been run yet."),
            (Some(_), None) => Some("These examples changed since they were last run, so their results are not shown."),
            _ => None,
        };
        let mut counts = Counts::default();
        for example in &plan.examples {
            // varar records an example under its name and the lines of its steps in this oath.
            let mut lines: Vec<usize> = example.steps.iter().filter(|step| step.doc_path.is_none()).map(|step| step.match_span.start_line).collect();
            lines.dedup();
            let outcome = fresh.and_then(|results| results.examples.get(&(example.name.clone(), lines)));
            let status = outcome.map_or(Status::NotRun, |outcome| outcome.status);
            let steps = example
                .steps
                .iter()
                .map(|step| &step.step_def)
                .chain(example.header_binding.iter().map(|binding| &binding.step_def));
            let kind = if steps.into_iter().any(|step| is_e2e(step)) { Kind::E2e } else { Kind::Core };
            counts.add(status, kind);
            let (from, to) = span(&example.span);
            push(path, from, to, Mark::Example(status));
            for step in &example.steps {
                let (from, to) = span(&step.match_span);
                push(step.doc_path.as_deref().unwrap_or(path), from, to, Mark::Step(kind, status));
            }
            if let Some(binding) = &example.header_binding {
                let (from, to) = span(&binding.match_span);
                push(path, from, to, Mark::Step(kind, status));
            }
            if let Some(failure) = outcome.and_then(|outcome| outcome.failure.as_ref()) {
                let doc = failure.doc_path.as_deref().unwrap_or(path);
                if let Some((from, to)) = failure.anchor {
                    push(doc, from, to, Mark::Anchor(failure.message.clone()));
                }
                for (from, to, actual) in &failure.cells {
                    push(doc, *from, *to, Mark::Cell(actual.clone()));
                }
            }
        }
        pages.push((path, banner, counts));
    }

    std::fs::create_dir_all(out)?;
    std::fs::write(out.join("style.css"), STYLE)?;
    let mut overview =
        String::from("<table class=\"overview\">\n<thead><tr><th>Page</th><th>Passed</th><th>Failed</th><th>Not run</th><th>e2e</th><th>core</th></tr></thead>\n<tbody>\n");
    for (path, banner, counts) in &pages {
        let source = sources[path.as_str()];
        let (title, body) = render_page(source, marks.get(path.as_str()).map_or(&[][..], Vec::as_slice));
        if counts.total() > 0 {
            let status = if counts.failed > 0 {
                "failed"
            } else if counts.not_run > 0 {
                "not-run"
            } else {
                "passed"
            };
            let _ = writeln!(
                overview,
                "<tr class=\"{status}\"><td><a href=\"{}\">{}</a></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                page_name(path),
                escape(&title),
                counts.passed,
                counts.failed,
                counts.not_run,
                counts.e2e,
                counts.core
            );
        }
        let mut main = String::new();
        // A page without examples has nothing to run, so it has no results either.
        if let (Some(banner), true) = (banner, counts.total() > 0) {
            let _ = writeln!(main, "<p class=\"banner\">{banner}</p>");
        }
        if counts.total() > 0 {
            main.push_str(&counts.summary());
        }
        main.push_str(&body);
        leave_room_for_overview(&mut main, &page_name(path));
        std::fs::write(out.join(page_name(path)), page(&title, &main))?;
    }
    overview.push_str("</tbody>\n</table>\n");
    // The index lists the results of every page after its own text.
    let index = out.join("index.html");
    if let Ok(html) = std::fs::read_to_string(&index) {
        std::fs::write(&index, html.replace(OVERVIEW_PLACEHOLDER, &format!("<h2 id=\"run-results\">Run results</h2>\n{overview}")))?;
    }
    Ok(())
}

const OVERVIEW_PLACEHOLDER: &str = "<!-- run results -->";

/// Leave a place for the overview of the run at the end of the index.
fn leave_room_for_overview(main: &mut String, page: &str) {
    if page == "index.html" {
        main.push_str(OVERVIEW_PLACEHOLDER);
    }
}

/// A whole page around the HTML of an oath.
fn page(title: &str, main: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} · BSG-001</title>
<link rel="stylesheet" href="style.css">
</head>
<body>
<header class="bar">
<a href="index.html">BSG-001 manual</a>
<span class="legend"><span class="badge passed">passed</span> <span class="badge failed">failed</span> <span class="badge not-run">not run</span> <span class="step e2e">e2e</span> <span class="step core">core</span></span>
</header>
<main>
{main}</main>
</body>
</html>
"#,
        title = escape(title)
    )
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn overlaps(range: &Range<usize>, other: &Range<usize>) -> bool {
    range.start < other.end && other.start < range.end
}

/// The worst status of the examples that overlap a range, when any does.
fn example_status(marks: &[Marked], range: &Range<usize>) -> Option<Status> {
    marks
        .iter()
        .filter(|marked| overlaps(&marked.range, range))
        .filter_map(|marked| match marked.mark {
            Mark::Example(status) => Some(status),
            _ => None,
        })
        .max()
}

/// The opening tag of the span that shows the inline marks, when there are any.
fn inline_span(marks: &[&Marked]) -> Option<String> {
    let mut classes = BTreeSet::new();
    let mut titles = Vec::new();
    let mut actual = None;
    let kind = marks
        .iter()
        .filter_map(|marked| if let Mark::Step(kind, _) = marked.mark { Some(kind) } else { None })
        .max();
    let status = marks
        .iter()
        .filter_map(|marked| if let Mark::Step(_, status) = marked.mark { Some(status) } else { None })
        .max();
    if let (Some(kind), Some(status)) = (kind, status) {
        classes.extend(["step", kind.class(), status.class()]);
    }
    for marked in marks {
        match &marked.mark {
            Mark::Anchor(message) => {
                classes.insert("anchor");
                titles.push(message.clone());
            }
            Mark::Cell(value) => {
                classes.insert("cell-failed");
                titles.push(format!("Actual: {value}"));
                actual = Some(value.clone());
            }
            _ => {}
        }
    }
    if classes.is_empty() {
        return None;
    }
    let mut tag = format!("<span class=\"{}\"", classes.into_iter().collect::<Vec<_>>().join(" "));
    if !titles.is_empty() {
        let _ = write!(tag, " title=\"{}\"", escape(&titles.join("\n")));
    }
    if let Some(actual) = actual {
        let _ = write!(tag, " data-actual=\"{}\"", escape(&actual));
    }
    tag.push('>');
    Some(tag)
}

fn inline_marks<'a>(marks: &'a [Marked], range: &Range<usize>) -> Vec<&'a Marked> {
    marks
        .iter()
        .filter(|marked| !matches!(marked.mark, Mark::Example(_)) && overlaps(&marked.range, range))
        .collect()
}

/// The text inside a span, wrapped in the span when it opens one.
fn wrapped<'a>(open: Option<String>, inner: Event<'a>, events: &mut Vec<Event<'a>>) {
    match open {
        Some(open) => {
            events.push(Event::InlineHtml(open.into()));
            events.push(inner);
            events.push(Event::InlineHtml("</span>".into()));
        }
        None => events.push(inner),
    }
}

/// An identifier for a heading, as GitHub makes it.
fn slug(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c),
            _ => None,
        })
        .collect()
}

/// A link to another oath points to its page instead.
fn page_link(destination: &str) -> Option<String> {
    if destination.contains("://") {
        return None;
    }
    let (path, fragment) = destination.split_once('#').map_or((destination, None), |(path, fragment)| (path, Some(fragment)));
    let stem = path.strip_suffix(".md")?;
    Some(match fragment {
        Some(fragment) => format!("{stem}.html#{fragment}"),
        None => format!("{stem}.html"),
    })
}

/// The title and HTML of an oath, with its marks shown.
pub fn render_page(source: &str, marks: &[Marked]) -> (String, String) {
    let input: Vec<(Event, Range<usize>)> = Parser::new_ext(source, Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH).into_offset_iter().collect();
    let mut events = Vec::with_capacity(input.len());
    let mut title = None;
    let mut used_ids = BTreeSet::new();
    let mut wrapped_blocks = Vec::new();
    let mut in_head = false;
    let mut row_status = None;
    for (index, (event, range)) in input.iter().enumerate() {
        match event {
            Event::Start(Tag::Paragraph | Tag::CodeBlock(_)) => {
                let status = example_status(marks, range);
                if let Some(status) = status {
                    events.push(Event::Html(format!("<div class=\"example {}\">", status.class()).into()));
                }
                wrapped_blocks.push(status.is_some());
                events.push(event.clone());
            }
            Event::End(TagEnd::Paragraph | TagEnd::CodeBlock) => {
                events.push(event.clone());
                if wrapped_blocks.pop() == Some(true) {
                    events.push(Event::Html("</div>\n".into()));
                }
            }
            Event::Start(Tag::TableHead) => {
                in_head = true;
                events.push(event.clone());
            }
            Event::End(TagEnd::TableHead) => {
                in_head = false;
                events.push(event.clone());
            }
            Event::Start(Tag::TableRow) => {
                row_status = example_status(marks, range);
                events.push(event.clone());
            }
            Event::Start(Tag::TableCell) => {
                events.push(event.clone());
                if let (false, Some(status)) = (in_head, row_status) {
                    events.push(Event::InlineHtml(format!("<span class=\"row {}\"></span>", status.class()).into()));
                }
            }
            Event::Start(Tag::Heading { level, id, classes, attrs }) => {
                let text: String = input[index + 1..]
                    .iter()
                    .take_while(|(event, _)| !matches!(event, Event::End(TagEnd::Heading(_))))
                    .filter_map(|(event, _)| match event {
                        Event::Text(text) | Event::Code(text) => Some(text.as_ref()),
                        _ => None,
                    })
                    .collect();
                if title.is_none() && *level == pulldown_cmark::HeadingLevel::H1 {
                    title = Some(text.clone());
                }
                let id = id.clone().unwrap_or_else(|| {
                    let base = slug(&text);
                    let mut id = base.clone();
                    let mut n = 0;
                    while used_ids.contains(&id) {
                        n += 1;
                        id = format!("{base}-{n}");
                    }
                    CowStr::from(id)
                });
                used_ids.insert(id.to_string());
                events.push(Event::Start(Tag::Heading {
                    level: *level,
                    id: Some(id),
                    classes: classes.clone(),
                    attrs: attrs.clone(),
                }));
            }
            Event::Start(Tag::Link { link_type, dest_url, title, id }) => {
                let dest_url = page_link(dest_url).map_or_else(|| dest_url.clone(), CowStr::from);
                events.push(Event::Start(Tag::Link {
                    link_type: *link_type,
                    dest_url,
                    title: title.clone(),
                    id: id.clone(),
                }));
            }
            Event::Text(text) if source.get(range.clone()) == Some(text.as_ref()) => {
                // The text is the source itself, so it can be cut exactly where each mark starts and ends.
                let overlapping = inline_marks(marks, range);
                let mut cuts: BTreeSet<usize> = [range.start, range.end].into();
                for marked in &overlapping {
                    cuts.extend([marked.range.start, marked.range.end].into_iter().filter(|cut| range.contains(cut)));
                }
                let cuts: Vec<usize> = cuts.into_iter().collect();
                for piece in cuts.windows(2).map(|pair| pair[0]..pair[1]) {
                    let covering: Vec<&Marked> = overlapping.iter().copied().filter(|marked| overlaps(&marked.range, &piece)).collect();
                    wrapped(inline_span(&covering), Event::Text(source[piece].to_owned().into()), &mut events);
                }
            }
            Event::Text(_) | Event::Code(_) => wrapped(inline_span(&inline_marks(marks, range)), event.clone(), &mut events),
            _ => events.push(event.clone()),
        }
    }
    let mut body = String::new();
    html::push_html(&mut body, events.into_iter());
    (title.unwrap_or_default(), body)
}

const STYLE: &str = r#":root {
  --bg: #fbfbf8;
  --fg: #1d1f21;
  --muted: #5f656b;
  --border: #d9dcd6;
  --code-bg: #f1f1ec;
  --pass: #1f7a3a;
  --pass-bg: #e6f3e9;
  --fail: #b3261e;
  --fail-bg: #fbe8e6;
  --not-run: #8a8f95;
  --e2e: #ece4fc;
  --core: #ddeefc;
  --tip-bg: #1d1f21;
  --tip-fg: #fbfbf8;
}
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    --bg: #17191b;
    --fg: #e4e6e8;
    --muted: #a1a7ad;
    --border: #34383c;
    --code-bg: #212427;
    --pass: #5cc27a;
    --pass-bg: #18301f;
    --fail: #f28b82;
    --fail-bg: #3a1d1b;
    --not-run: #7b8187;
    --e2e: #3a2f57;
    --core: #1f3a52;
    --tip-bg: #e4e6e8;
    --tip-fg: #17191b;
  }
}
:root[data-theme="dark"] {
  --bg: #17191b;
  --fg: #e4e6e8;
  --muted: #a1a7ad;
  --border: #34383c;
  --code-bg: #212427;
  --pass: #5cc27a;
  --pass-bg: #18301f;
  --fail: #f28b82;
  --fail-bg: #3a1d1b;
  --not-run: #7b8187;
  --e2e: #3a2f57;
  --core: #1f3a52;
  --tip-bg: #e4e6e8;
  --tip-fg: #17191b;
}
* { box-sizing: border-box; }
body { margin: 0; background: var(--bg); color: var(--fg); font: 16px/1.6 system-ui, -apple-system, "Segoe UI", sans-serif; }
a { color: inherit; }
.bar { position: sticky; top: 0; z-index: 1; display: flex; flex-wrap: wrap; gap: .5rem 1rem; align-items: center; justify-content: space-between; padding: .5rem 16px; background: var(--bg); border-bottom: 1px solid var(--border); font-size: .875rem; }
.bar > a { font-weight: 600; text-decoration: none; }
main { max-width: 50rem; margin: 0 auto; padding: 1rem 16px 4rem; }
h1, h2, h3 { line-height: 1.25; }
code { font: .9em ui-monospace, "SFMono-Regular", Menlo, monospace; background: var(--code-bg); padding: .1em .25em; border-radius: 3px; }
pre { background: var(--code-bg); padding: .75rem 1rem; border-radius: 4px; overflow-x: auto; }
pre code { background: none; padding: 0; }
table { display: block; overflow-x: auto; border-collapse: collapse; margin: 1rem 0; }
th, td { border: 1px solid var(--border); padding: .25rem .6rem; text-align: left; vertical-align: top; }
details { border: 1px solid var(--border); border-radius: 4px; padding: .25rem 1rem; margin: 1rem 0; }
summary { cursor: pointer; font-weight: 600; }
.example { border-left: 4px solid var(--not-run); padding-left: .75rem; margin-left: -1rem; }
.example.passed { border-left-color: var(--pass); }
.example.failed { border-left-color: var(--fail); background: var(--fail-bg); }
.example > p, .example > pre { margin: .5rem 0; }
tr:has(.row.passed) td:first-child { box-shadow: inset 4px 0 var(--pass); }
tr:has(.row.not-run) td:first-child { box-shadow: inset 4px 0 var(--not-run); }
tr:has(.row.failed) td { background: var(--fail-bg); }
tr:has(.row.failed) td:first-child { box-shadow: inset 4px 0 var(--fail); }
.step { border-radius: 3px; padding: 0 .1em; -webkit-box-decoration-break: clone; box-decoration-break: clone; }
.step.e2e { background: var(--e2e); }
.step.core { background: var(--core); }
.step.passed { text-decoration: underline 2px var(--pass); text-underline-offset: 3px; }
.step.failed { text-decoration: underline 2px var(--fail); text-underline-offset: 3px; }
.anchor { text-decoration: underline wavy var(--fail) !important; text-decoration-skip-ink: none; cursor: help; }
.cell-failed { position: relative; text-decoration: underline wavy var(--fail); text-decoration-skip-ink: none; cursor: help; }
.cell-failed:hover::after { content: "Actual: " attr(data-actual); position: absolute; left: 0; top: 100%; z-index: 2; white-space: pre; padding: .25rem .5rem; border-radius: 4px; background: var(--tip-bg); color: var(--tip-fg); font-size: .8rem; }
.legend, .summary { display: flex; flex-wrap: wrap; gap: .4rem; align-items: center; }
.badge { border-radius: 999px; padding: 0 .55em; font-size: .8rem; border: 1px solid var(--border); }
.badge.passed { color: var(--pass); border-color: var(--pass); }
.badge.failed { color: var(--fail); border-color: var(--fail); }
.badge.not-run { color: var(--not-run); border-color: var(--not-run); }
.badge.e2e { background: var(--e2e); }
.badge.core { background: var(--core); }
.banner { padding: .5rem .75rem; border-radius: 4px; border: 1px solid var(--not-run); color: var(--muted); }
.overview tr.failed td { background: var(--fail-bg); }
.overview td:not(:first-child) { text-align: right; }
"#;
