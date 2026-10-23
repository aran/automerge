use super::{SampledBenchmark, SeriesBenchmark};
use benchmark_battery::automerge::marks::{ExpandMark, Mark};
use benchmark_battery::automerge::transaction::Transactable;
use benchmark_battery::automerge::{Automerge, ObjId, ObjType, ReadDoc, ROOT};
use benchmark_battery::{rand, text_splice_100};
use std::cmp::{max, min};
use std::hint::black_box;

const N: u64 = 100_000;

const PARAGRAPHS: usize = 4_000;

pub fn benchmarks() -> Vec<SampledBenchmark> {
    vec![
        SampledBenchmark::no_setup("marks", "marks/add_mark", add_mark),
        SampledBenchmark::no_setup("marks", "marks/get_marks_at_end", get_marks_at_end),
        splice_with_marks(),
        splice_without_marks(),
        SampledBenchmark::no_setup(
            "marks",
            "marks/marks_of_expanding_paragraphs",
            marks_of_expanding_paragraphs,
        ),
    ]
}

fn get_marks_at_end() -> Box<dyn FnMut()> {
    let (doc, text) = doc_and_text(10);
    let index = doc.length(&text) - 1;
    Box::new(move || {
        black_box(doc.get_marks(&text, index, None).unwrap());
    })
}

pub fn series() -> Vec<SeriesBenchmark> {
    vec![
        SeriesBenchmark {
            group: "marks",
            name: "marks/append_paragraph",
            steps: PARAGRAPHS,
            setup: || append_paragraphs(ExpandMark::None, false),
        },
        SeriesBenchmark {
            group: "marks",
            name: "marks/append_paragraph_expanding",
            steps: PARAGRAPHS,
            setup: || append_paragraphs(ExpandMark::Both, false),
        },
        SeriesBenchmark {
            group: "marks",
            name: "marks/append_paragraph_expanding_unmarked",
            steps: PARAGRAPHS,
            setup: || append_paragraphs(ExpandMark::Both, true),
        },
    ]
}

fn append_paragraphs(expand: ExpandMark, unmark: bool) -> Box<dyn FnMut(usize)> {
    let (mut doc, text) = empty_text();
    Box::new(move |step| {
        let mut tx = doc.transaction();
        append_paragraph(&mut tx, &text, step, expand, unmark);
        tx.commit();
    })
}

fn marks_of_expanding_paragraphs() -> Box<dyn FnMut()> {
    let (mut doc, text) = empty_text();
    let mut tx = doc.transaction();
    for step in 0..PARAGRAPHS {
        append_paragraph(&mut tx, &text, step, ExpandMark::Both, true);
    }
    tx.commit();
    Box::new(move || {
        black_box(doc.marks(&text).unwrap());
    })
}

fn empty_text() -> (Automerge, ObjId) {
    let mut doc = Automerge::new();
    let mut tx = doc.transaction();
    let text = tx.put_object(ROOT, "text", ObjType::Text).unwrap();
    tx.commit();
    (doc, text)
}

fn append_paragraph<T: Transactable>(
    tx: &mut T,
    text: &ObjId,
    step: usize,
    expand: ExpandMark,
    unmark: bool,
) {
    let at = tx.length(text);
    tx.split_block(text, at).unwrap();
    let bold = format!("heading {step}");
    let n = bold.chars().count();
    tx.splice_text(text, at + 1, 0, &bold).unwrap();
    tx.mark(
        text,
        Mark::new("bold".into(), true, at + 1, at + 1 + n),
        expand,
    )
    .unwrap();
    let rest = " and then a plain sentence of some twenty words or so that runs on to the end of the paragraph";
    let m = rest.chars().count();
    tx.splice_text(text, at + 1 + n, 0, rest).unwrap();
    if unmark {
        tx.unmark(text, "bold", at + 1 + n, at + 1 + n + m, expand)
            .unwrap();
    }
}

fn add_mark() -> Box<dyn FnMut()> {
    let mut doc = text_splice_100(N);
    Box::new(move || {
        add_random_marks(&mut doc, 1);
    })
}

fn add_random_marks(d: &mut Automerge, num: usize) {
    let (_, text) = d.get(ROOT, "content").unwrap().unwrap();
    let mut tx = d.transaction();
    let len = tx.length(&text);
    for i in 0..num {
        let a = rand() % len;
        let b = rand() % len;
        let mark = Mark::new(format!("mark{}", i), true, min(a, b), max(a, b));
        tx.mark(&text, mark, ExpandMark::Both).unwrap();
    }
    tx.commit();
}

fn splice_with_marks() -> SampledBenchmark {
    SampledBenchmark::batched(
        "marks",
        "marks/splice_with_marks",
        || doc_and_text(1_000),
        |(mut doc, text)| {
            for _ in 0..1000 {
                let mut tx = doc.transaction();
                let pos = rand() % tx.length(&text);
                tx.splice_text(&text, pos, 0, "XXX").unwrap();
                tx.commit();
            }
            (doc, text)
        },
    )
}

fn splice_without_marks() -> SampledBenchmark {
    SampledBenchmark::batched(
        "marks",
        "marks/splice_without_marks",
        || doc_and_text(0),
        |(mut doc, text)| {
            for _ in 0..1000 {
                let mut tx = doc.transaction();
                let pos = rand() % tx.length(&text);
                tx.splice_text(&text, pos, 1, "XXX").unwrap();
                tx.commit();
            }
            (doc, text)
        },
    )
}

fn doc_and_text(marks: usize) -> (Automerge, ObjId) {
    let mut doc = text_splice_100(N);
    add_random_marks(&mut doc, marks);
    let (_, text) = doc.get(ROOT, "content").unwrap().unwrap();
    (doc, text)
}
