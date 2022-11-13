use std::{fs::File, io::Write};

use newtabgen::config::{Config, Link, Page, Section};

use lipsum::lipsum_words;
use rand::prelude::*;

pub(crate) fn run() {
    let config = gen();
    let json = serde_json::ser::to_string(&config).expect("failed to serialize config");
    let mut file = File::create("generated.json").expect("failed to create file");
    file.write_all(json.as_bytes())
        .expect("failed to write file");
}

fn gen() -> Config {
    let mut rng = thread_rng();
    Config {
        pages: gen_pages(&mut rng),
        ..Config::default()
    }
}

fn gen_pages(rng: &mut ThreadRng) -> Vec<Page> {
    let n = rng.gen_range(1..10);
    let mut vec = Vec::<Page>::new();
    for _ in 1..n {
        vec.push(Page {
            name: lipsum_words(rng.gen_range(1..10)),
            sections: gen_sections(rng),
            icon: "image_not_supported".into(),
            icon_style: "outlined".into(),
        });
    }
    vec
}

fn gen_sections(rng: &mut ThreadRng) -> Vec<Section> {
    let n = rng.gen_range(1..10);
    let mut vec = Vec::<Section>::new();
    for _ in 1..n {
        vec.push(Section {
            name: lipsum_words(rng.gen_range(1..10)),
            links: gen_links(rng),
        });
    }
    vec
}

fn gen_links(rng: &mut ThreadRng) -> Vec<Link> {
    let n = rng.gen_range(1..10);
    let mut vec = Vec::<Link>::new();
    let urls = [
        "https://crates.io/",
        "https://www.rust-lang.org/",
        "https://docs.rs/",
        "https://stackoverflow.com/",
        "https://duckduckgo.com/",
        "https://fsf.org/",
        "https://eff.org/",
        "https://codeberg.org/",
        "https://css-tricks.com/",
        "https://developer.mozilla.org/",
    ];
    for _ in 1..n {
        vec.push(Link {
            name: lipsum_words(rng.gen_range(1..10)),
            url: (*urls.choose(rng).unwrap()).into(),
        });
    }
    vec
}
