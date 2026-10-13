use super::*;

#[test]
fn svg_carries_explicit_size_and_root_tag() {
    let svg = album_svg(AlbumId(0));
    assert!(svg.starts_with(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"512\" height=\"512\" viewBox=\"0 0 512 512\">"
    ));
    assert!(svg.contains("width=\"512\""));
    assert!(svg.contains("height=\"512\""));
}

#[test]
fn same_id_produces_identical_bytes() {
    assert_eq!(album_svg(AlbumId(5)), album_svg(AlbumId(5)));
}

#[test]
fn catalog_albums_get_distinct_covers() {
    let mut seen: Vec<String> = Vec::new();
    for id in 0..8u32 {
        let svg = album_svg(AlbumId(id));
        assert!(
            !seen.contains(&svg),
            "albums 0..{id} produced duplicate cover bytes"
        );
        seen.push(svg);
    }
}

#[test]
fn generated_bytes_are_pure_ascii() {
    for id in 0..8u32 {
        assert!(album_svg(AlbumId(id)).is_ascii(), "id {id} injected markup");
    }
}
