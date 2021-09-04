use arrayvec::ArrayVec;
use atom_syndication::Feed;
use clap::{crate_authors, crate_description, crate_name, crate_version, Arg, Error, ErrorKind};
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::{
    blocking::{Client, Response},
    header::ACCEPT,
};

fn extract_id<'a, const N: usize>(re_arr: &ArrayVec<Regex, N>, pat: &'a str) -> &'a str {
    let m = re_arr
        .iter()
        .filter_map(|re| re.captures(&pat))
        .map(|m| m.get(0).unwrap().as_str())
        .collect::<ArrayVec<_, 1>>();
    if m.len() == 0 {
        Error::with_description("Invalid DOI or arXiv ID!", ErrorKind::ValueValidation).exit();
    }
    let id = m.get(0).unwrap().trim_end_matches("/");
    id
}

fn request_doi(client: &Client, id: &str) -> Result<Response, reqwest::Error> {
    println!("Making request to {}", &format!("https://doi.org/{}", id));
    client
        .get(&format!("https://doi.org/{}", id))
        .header(ACCEPT, "text/bibliography; style=bibtex")
        .send()
}

fn request_arxiv(client: &Client, id: &str) -> Result<Response, reqwest::Error> {
    client
        .get(&format!("http://export.arxiv.org/api/query?id_list={}", id))
        .send()
}

fn format_doi(input: &str) -> String {
    DOI_FMT
        .replace_all(input.trim(), ",\n  $1")
        .replace("}}", "}\n}")
}

fn handle_response(res: Result<Response, reqwest::Error>, idtype: IdType) {
    if res.is_err() {
        Error::with_description("Failed to get bibtex information!", ErrorKind::InvalidValue);
    }
    let res = res.unwrap().text_with_charset("utf-8").unwrap();
    match idtype {
        IdType::Doi => println!("{}", format_doi(&res)),
        IdType::Arxiv => println!("{:#?}", res.parse::<Feed>().unwrap()),
    }
}

lazy_static! {
    pub static ref DOI_IDENT_RE: Regex = Regex::new(r"doi(?::|.org)").unwrap();
    pub static ref DOI_RE: ArrayVec<Regex, 2> = [r"10.\d{4,9}/[-\._;()/:\w\d]+", r"10.1002/[^\s]+"]
        .iter()
        .map(|re| Regex::new(re).unwrap())
        .collect();
    pub static ref DOI_FMT: Regex = Regex::new(r",(\s?\w+=\{.+?\})").unwrap();
    pub static ref ARXIV_IDENT_RE: Regex = Regex::new(r"(?i)arxiv(?-i)(?::|.org)").unwrap();
    pub static ref ARXIV_RE: ArrayVec<Regex, 2> = [
        r"\d{4}\.\d{4,5}(?:v\d+)?",
        r"[a-z]+(?:-[a-z]+)?/\d{7}(?:v\d+)?",
    ]
    .iter()
    .map(|re| Regex::new(re).unwrap())
    .collect();
}

enum IdType {
    Doi,
    Arxiv,
}

fn main() {
    let matches = clap::App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(
            Arg::with_name("input")
                .help("DOI(s) or arXiv identifier(s) to search for, separated by spaces.")
                .required(true)
                .index(1)
                .min_values(1),
        )
        .get_matches();

    let client = Client::new();

    let pats = if let Some(pats) = matches.values_of("input") {
        pats.collect::<Vec<_>>()
    } else {
        Error::with_description("Missing arguments!", ErrorKind::MissingRequiredArgument).exit();
    };

    for pat in pats {
        let (id, idtype) =
            if DOI_IDENT_RE.is_match(&pat) || DOI_RE.iter().any(|re| re.is_match(&pat)) {
                (extract_id(&DOI_RE, pat), IdType::Doi)
            } else if ARXIV_IDENT_RE.is_match(&pat) || ARXIV_RE.iter().any(|re| re.is_match(&pat)) {
                (extract_id(&ARXIV_RE, pat), IdType::Arxiv)
            } else {
                Error::with_description(
                    "Please enter a valid DOI or arXiv ID!",
                    ErrorKind::InvalidValue,
                )
                .exit();
            };
        let res = match idtype {
            IdType::Doi => {
                println!("matched DOI: {}", id);
                request_doi(&client, id)
            }
            IdType::Arxiv => {
                println!("matched arXiv ID: {}", id);
                request_arxiv(&client, id)
            }
        };
        handle_response(res, idtype);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_arxiv_id() {
        let inputs = vec![
            "arxiv:2105.11572",
            "https://arxiv.org/abs/1912.02599v2",
            "2105.11572",
            "https://arxiv.org/abs/math/0506203",
            "math/0506203",
            "hep-th/9910001",
            "https://arxiv.org/abs/hep-th/9910001v2",
        ];

        let extracted_ids = inputs
            .iter()
            .map(|pat| extract_id(&ARXIV_RE, pat))
            .collect::<Vec<_>>();

        let true_ids = vec![
            "2105.11572",
            "1912.02599v2",
            "2105.11572",
            "math/0506203",
            "math/0506203",
            "hep-th/9910001",
            "hep-th/9910001v2",
        ];

        assert_eq!(extracted_ids, true_ids);
    }
}
