use arrayvec::ArrayVec;
use clap::{crate_authors, crate_description, crate_name, crate_version, Arg, Error, ErrorKind};
use regex::Regex;

fn main() {
    let matches = clap::App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(
            Arg::with_name("input")
                .help("DOI to search for")
                .required(true)
                .index(1),
        )
        .get_matches();

    let doi = Regex::new(r"doi.org").unwrap();
    let mut doi_re = [r"10.\d{4,9}/[-._;()/:A-Z0-9]+", r"10.1002/[^\s]+"]
        .iter()
        .map(|re| Regex::new(re).unwrap());

    let arxiv = Regex::new(r"arxiv.org").unwrap();
    let arxiv_re = Regex::new(r"(\d{4}.\d{4,5}|[a-z\-]+(\.[A-Z]{2})?/\d{7})(v\d+)?").unwrap();

    if let Some(pat) = matches.value_of("input") {
        if doi.is_match(&pat) || doi_re.any(|re| re.is_match(&pat)) {
            let m = doi_re
                .filter_map(|re| re.captures(&pat))
                .map(|m| m.get(0).unwrap().as_str())
                .collect::<ArrayVec<_, 1>>();
            if m.len() == 0 {
                Error::with_description("Invalid DOI!", ErrorKind::ValueValidation).exit();
            }
            println!("DOI matched: {}", m.get(0).unwrap());
        } else if arxiv.is_match(&pat) || arxiv_re.is_match(&pat) {
            let m = arxiv_re.captures(&pat);
            if m.is_none() {
                Error::with_description("Invalid arXiv ID!", ErrorKind::ValueValidation).exit();
            }
            let m = m.unwrap();
            if let Some(v) = m.get(1) {
                println!("arXiv ID matched: {}", v.as_str());
            } else {
                Error::with_description(
                    &format!("Invalid arXiv ID: {}", m.get(0).unwrap().as_str()),
                    ErrorKind::ValueValidation,
                )
                .exit();
            }
        } else {
            Error::with_description(
                "Please enter a valid DOI or arXiv ID!",
                ErrorKind::InvalidValue,
            )
            .exit();
        }
    }
}
