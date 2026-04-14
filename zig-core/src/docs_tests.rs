use super::*;

#[test]
fn get_existing_topic() {
    assert!(get("zug").is_some());
    assert!(get("patterns").is_some());
    assert!(get("variables").is_some());
    assert!(get("conditions").is_some());
    assert!(get("memory").is_some());
}

#[test]
fn get_unknown_topic_returns_none() {
    assert!(get("nonexistent").is_none());
    assert!(get("").is_none());
}

#[test]
fn command_topics_are_not_docs() {
    // Command topics live under `zig man`, not `zig docs`.
    for topic in ["zig", "run", "listen", "workflow", "describe", "validate"] {
        assert!(
            get(topic).is_none(),
            "'{topic}' should be a manpage, not a docs topic"
        );
    }
}

#[test]
fn all_pages_are_nonempty() {
    for (topic, _) in TOPICS {
        let content = get(topic).unwrap_or_else(|| panic!("missing docs page for '{topic}'"));
        assert!(!content.is_empty(), "docs page for '{topic}' is empty");
    }
}

#[test]
fn all_pages_start_with_heading() {
    for (topic, _) in TOPICS {
        let content = get(topic).unwrap();
        assert!(
            content.starts_with('#'),
            "docs page for '{topic}' should start with a markdown heading"
        );
    }
}

#[test]
fn list_topics_contains_all_entries() {
    let listing = list_topics();
    for (topic, description) in TOPICS {
        assert!(
            listing.contains(topic),
            "listing should contain topic '{topic}'"
        );
        assert!(
            listing.contains(description),
            "listing should contain description '{description}'"
        );
    }
}

#[test]
fn list_topics_shows_usage() {
    let listing = list_topics();
    assert!(listing.contains("zig docs <topic>"));
}

#[test]
fn topics_list_matches_get() {
    for (topic, _) in TOPICS {
        assert!(
            get(topic).is_some(),
            "TOPICS lists '{topic}' but get() returns None"
        );
    }
}
