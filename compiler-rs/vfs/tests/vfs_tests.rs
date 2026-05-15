use infra::Span;
use vfs::{FileWatcher, SourceMap, Vfs};

#[test]
fn end_to_end_vfs_workflow() {
    let mut vfs = Vfs::new(SourceMap::new());
    let content = "constant = `hello`\n";
    let id = vfs
        .add_physical_file("src/main.fer", content.into())
        .unwrap();

    // Physical read
    assert_eq!(vfs.read(id), content);
    // Path lookup
    let path = vfs.source_map().path(id).unwrap();
    assert_eq!(path.as_str(vfs.source_map().interner()), "src/main.fer");
    assert_eq!(vfs.source_map().id_of(path), Some(id));

    // Span resolution
    let span = Span::new(11, 11);
    let (line, col) = vfs.source_map().resolve(id, span).unwrap();
    assert_eq!(line, 1);
    assert_eq!(col, 11);

    // Overlay
    vfs.set_overlay(id, "overlay content".into());
    assert_eq!(vfs.read(id), "overlay content");
    vfs.clear_overlay(id);
    assert_eq!(vfs.read(id), content);

    // File watcher
    let watcher = FileWatcher::new();
    assert!(!watcher.has_changes());
}

#[test]
fn rejects_kebab_case_violations() {
    let mut vfs = Vfs::new(SourceMap::new());
    assert!(
        vfs.add_physical_file("src/user_repo.fer", "".into())
            .is_none()
    );
    assert!(
        vfs.add_physical_file("src/UserRepo.fer", "".into())
            .is_none()
    );
}
