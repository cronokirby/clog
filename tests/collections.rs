use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new() -> Self {
        let nonce = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "clog-collections-{}-{timestamp}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create temporary fixture directory");
        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn write(path: impl AsRef<Path>, contents: &str) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().expect("fixture file has a parent"))
        .expect("create fixture parent directory");
    fs::write(path, contents).expect("write fixture file");
}

fn fixture(template: &str) -> TempDir {
    let fixture = TempDir::new();
    write(fixture.path.join("templates/index.html"), template);
    fs::create_dir_all(fixture.path.join("content")).expect("create content directory");
    fixture
}

fn run(fixture: &TempDir) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_clog"))
        .arg(&fixture.path)
        .arg(fixture.path.join("output"))
        .output()
        .expect("run clog")
}

fn output(fixture: &TempDir, page: &str) -> String {
    fs::read_to_string(fixture.path.join("output").join(page)).expect("read generated page")
}

#[test]
fn collections_render_scalars_and_lists_and_create_deduplicated_backlinks() {
    let fixture = fixture(
        "C:{% for collection in collections %}[{{ collection }}]{% else %}<none>{% endfor %}\n\
         A:{% for author in authors %}[{{ author }}]{% else %}<none>{% endfor %}\n\
         B:{% for backlink in backlinks %}[{{ backlink.title }}|{{ backlink.link }}]{% else %}<none>{% endfor %}\n\
         BODY:{{ body }}",
    );
    write(
        fixture.path.join("content/Person.md"),
        "---\ntitle: Person\n---\nPerson body\n",
    );
    write(
        fixture.path.join("content/Scalar.md"),
        "---\ntitle: Scalar\nauthors: \"[[Person|Writer]]\"\ncollections: \"[[Person|Curator]]\"\n---\nScalar body\n",
    );
    write(
        fixture.path.join("content/Listed.md"),
        "---\ntitle: Listed\ncollections:\n  - \"[[Person|Member]]\"\n  - Plain collection\n  - \"[[Missing|Fallback]]\"\n  - \"[[Person]]\"\n---\nListed body\n",
    );
    write(
        fixture.path.join("content/Draft.md"),
        "---\ntitle: Draft\ndraft: true\ncollections: \"[[Person|Hidden]]\"\n---\nDraft body\n",
    );

    let result = run(&fixture);
    assert!(
        result.status.success(),
        "clog failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let scalar = output(&fixture, "scalar.html");
    assert!(scalar.contains("C:[<a href=\"/person.html\">Curator</a>]"));
    assert!(scalar.contains("A:[<a href=\"/person.html\">Writer</a>]"));

    let listed = output(&fixture, "listed.html");
    assert!(listed.contains("[<a href=\"/person.html\">Member</a>]"));
    assert!(listed.contains("[Plain collection]"));
    assert!(listed.contains("[Fallback]"));
    assert!(listed.contains("[<a href=\"/person.html\">Person</a>]"));

    let person = output(&fixture, "person.html");
    assert_eq!(person.matches("[Scalar|/scalar.html]").count(), 1);
    assert_eq!(person.matches("[Listed|/listed.html]").count(), 1);
    assert!(!person.contains("Draft|/draft.html"));
    assert!(!fixture.path.join("output/draft.html").exists());
}

#[test]
fn absent_null_and_empty_collections_do_not_render_a_section() {
    let fixture = fixture(
        "{% if collections %}COLLECTIONS:{% for collection in collections %}[{{ collection }}]{% endfor %}{% endif %}\n\
         AUTHORS:{% for author in authors %}[{{ author }}]{% else %}<none>{% endfor %}\n\
         BODY:{{ body }}",
    );
    write(
        fixture.path.join("content/Missing.md"),
        "---\ntitle: Missing\n---\nMissing body\n",
    );
    write(
        fixture.path.join("content/Null.md"),
        "---\ntitle: Null\ncollections: null\n---\nNull body\n",
    );
    write(
        fixture.path.join("content/Empty.md"),
        "---\ntitle: Empty\ncollections: []\n---\nEmpty body\n",
    );

    let result = run(&fixture);
    assert!(
        result.status.success(),
        "clog failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    for page in ["missing.html", "null.html", "empty.html"] {
        let html = output(&fixture, page);
        assert!(
            !html.contains("COLLECTIONS:"),
            "unexpected section in {page}"
        );
        assert!(html.contains("AUTHORS:<none>"));
    }
}

#[test]
fn invalid_collection_shapes_are_rejected() {
    for (name, collections) in [("Number", "42"), ("Nested", "[[Person]]")] {
        let fixture = fixture("{{ body }}");
        write(
            fixture.path.join("content/Invalid.md"),
            &format!("---\ntitle: {name}\ncollections: {collections}\n---\nInvalid body\n"),
        );

        let result = run(&fixture);
        assert!(
            !result.status.success(),
            "collections: {collections} unexpectedly succeeded"
        );
    }
}
