use automerge::transaction::Transactable;
use automerge::{ActorId, AutoCommit, Automerge, ChangeHash, ObjType, ReadDoc, ROOT};

fn hashes(doc: &Automerge, since: &[ChangeHash]) -> Vec<ChangeHash> {
    doc.get_changes(since).iter().map(|c| c.hash()).collect()
}

fn assert_reloads(doc: &mut AutoCommit) {
    let expected = hashes(doc.document(), &[]);
    let loaded = Automerge::load(&doc.save()).unwrap();
    assert_eq!(loaded.get_heads(), doc.get_heads());
    assert_eq!(hashes(&loaded, &[]), expected);
    for change in expected.iter().step_by(3) {
        assert_eq!(
            hashes(&loaded, &[*change]),
            hashes(doc.document(), &[*change])
        );
    }
    let mut applied = Automerge::new();
    applied.apply_changes(loaded.get_changes(&[])).unwrap();
    assert_eq!(applied.get_heads(), doc.get_heads());
}

fn paragraphs(doc: &mut AutoCommit, count: usize) {
    let text = doc.put_object(ROOT, "text", ObjType::Text).unwrap();
    let mut len = 0;
    for p in 0..count {
        let block = doc.split_block(&text, len).unwrap();
        doc.put(&block, "type", "paragraph").unwrap();
        len += 1;
        let words: String = (0..40)
            .map(|i| (b'a' + ((p + i) % 26) as u8) as char)
            .collect();
        doc.splice_text(&text, len, 0, &words).unwrap();
        len += words.len();
    }
}

#[test]
fn reloads_change_whose_text_waits_for_block_properties() {
    let mut doc = AutoCommit::new();
    paragraphs(&mut doc, 120);
    doc.commit();
    assert_reloads(&mut doc);
}

#[test]
fn reloads_change_whose_overwrites_wait_for_another_object() {
    let mut doc = AutoCommit::new();
    let list = doc.put_object(ROOT, "list", ObjType::List).unwrap();
    let map = doc.put_object(ROOT, "map", ObjType::Map).unwrap();
    doc.put(&map, "x", 1).unwrap();
    doc.insert(&list, 0, 0).unwrap();
    doc.insert(&list, 1, 0).unwrap();
    for i in 0..3000 {
        doc.put(&list, 0, i).unwrap();
    }
    for i in 0..500 {
        doc.insert(&list, 2 + i, i as i64).unwrap();
    }
    doc.delete(&list, 1).unwrap();
    doc.put(&map, "x", 2).unwrap();
    doc.commit();
    assert_reloads(&mut doc);
}

#[test]
fn reloads_change_of_scattered_inserts() {
    let mut doc = AutoCommit::new();
    let text = doc.put_object(ROOT, "text", ObjType::Text).unwrap();
    let mut rng: u64 = 17;
    for i in 0..6000u64 {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        let len = doc.length(&text) as u64;
        let pos = (rng % (len + 1)) as usize;
        if i % 7 == 6 && len > 0 {
            doc.splice_text(&text, pos.min(len as usize - 1), 1, "")
                .unwrap();
        } else {
            doc.splice_text(&text, pos, 0, "ab").unwrap();
        }
    }
    doc.commit();
    assert_reloads(&mut doc);
}

#[test]
fn reloads_large_changes_from_several_actors() {
    let mut doc = AutoCommit::new().with_actor(ActorId::from([1u8; 16]));
    paragraphs(&mut doc, 60);
    doc.commit();
    let mut other = doc.fork().with_actor(ActorId::from([2u8; 16]));
    let text = doc.get(ROOT, "text").unwrap().unwrap().1;
    for i in 0..40 {
        let block = other.split_block(&text, i * 30).unwrap();
        other.put(&block, "type", "heading").unwrap();
        other
            .splice_text(&text, i * 30 + 1, 0, "inserted words")
            .unwrap();
    }
    other.commit();
    for i in 0..40 {
        doc.splice_text(&text, i * 20, 0, "local edit").unwrap();
    }
    doc.commit();
    doc.merge(&mut other).unwrap();
    paragraphs(&mut doc, 30);
    doc.commit();
    assert_reloads(&mut doc);
}
