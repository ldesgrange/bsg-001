//! The report that shows the oaths with the results of a run.

#[path = "support/report.rs"]
#[allow(dead_code)]
mod report;

use report::{Kind, Mark, Marked, Status, render_page};

/// The byte range of the first occurrence of `text` in `source`.
fn range_of(source: &str, text: &str) -> std::ops::Range<usize> {
    let start = source.find(text).expect("the text is in the source");
    start..start + text.len()
}

#[test]
fn a_sentence_that_matched_a_step_is_shown_with_its_kind_and_status() {
    let source = "# Title\n\nSome prose.\nA device is stopped.\n";
    let marks = [Marked {
        range: range_of(source, "A device is stopped."),
        mark: Mark::Step(Kind::E2e, Status::Passed),
    }];

    let (title, html) = render_page(source, &marks);

    assert_eq!(title, "Title");
    assert!(html.contains("Some prose.\n<span class=\"e2e passed step\">A device is stopped.</span>"), "{html}");
}

#[test]
fn a_paragraph_of_an_example_is_framed_with_the_status_of_the_example() {
    let source = "Prose.\n\nA device is stopped.\n";
    let marks = [Marked {
        range: range_of(source, "A device is stopped."),
        mark: Mark::Example(Status::Failed),
    }];

    let (_, html) = render_page(source, &marks);

    assert!(
        html.starts_with("<p>Prose.</p>\n<div class=\"example failed\">\n<p>A device is stopped.</p>\n</div>"),
        "{html}"
    );
}

#[test]
fn a_failed_cell_shows_its_actual_value() {
    let source = "| a | b |\n| --- | --- |\n| 1 | 2 |\n";
    let marks = [
        Marked {
            range: range_of(source, "| 1 | 2 |"),
            mark: Mark::Example(Status::Failed),
        },
        Marked {
            range: range_of(source, "2 |"),
            mark: Mark::Cell("3".to_owned()),
        },
    ];

    let (_, html) = render_page(source, &marks);

    assert!(html.contains("<td><span class=\"row failed\"></span>1</td>"), "{html}");
    assert!(html.contains("<span class=\"cell-failed\" title=\"Actual: 3\" data-actual=\"3\">2</span>"), "{html}");
}

#[test]
fn a_link_to_an_oath_points_to_its_page_and_headings_can_be_linked_to() {
    let source = "## Choosing the device\n\nSee [running](run.md#a-run), [the vendor](https://example.com/a.md).\n";

    let (_, html) = render_page(source, &[]);

    assert!(html.contains("<h2 id=\"choosing-the-device\">"), "{html}");
    assert!(html.contains("href=\"run.html#a-run\""), "{html}");
    assert!(html.contains("href=\"https://example.com/a.md\""), "{html}");
}
