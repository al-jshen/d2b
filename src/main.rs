use arrayvec::ArrayVec;
use async_recursion::async_recursion;
use atom_syndication::Feed;
use chrono::Datelike;
use clap::{crate_authors, crate_description, crate_name, crate_version, Arg, Error, ErrorKind};
use futures::{stream::FuturesUnordered, StreamExt};
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::{
    header::ACCEPT,
    {Client, Response},
};

fn extract_id<const N: usize>(re_arr: &ArrayVec<Regex, N>, pat: &str) -> String {
    let m = re_arr
        .iter()
        .filter_map(|re| re.captures(&pat))
        .map(|m| m.get(0).unwrap().as_str())
        .collect::<ArrayVec<_, 1>>();
    if m.len() == 0 {
        Error::with_description("Invalid DOI or arXiv ID!", ErrorKind::ValueValidation).exit();
    }
    let id = m.get(0).unwrap().trim_end_matches("/").to_owned();
    id
}

async fn request_info(id: &str, idtype: IdType) -> Result<Response, reqwest::Error> {
    // println!("Making request to {}", &format!("https://doi.org/{}", id));
    match idtype {
        IdType::Doi => {
            CLIENT
                .get(&format!("https://doi.org/{}", id))
                .header(ACCEPT, "text/bibliography; style=bibtex")
                .send()
                .await
        }
        IdType::Arxiv => {
            CLIENT
                .get(&format!("http://export.arxiv.org/api/query?id_list={}", id))
                .send()
                .await
        }
    }
}

fn print_doi(input: &str) -> String {
    DOI_FMT
        .replace_all(input.trim(), ",\n  $1")
        .replace("}}", "}\n}")
}

async fn print_arxiv(input: &Feed) -> String {
    if input.entries().is_empty() {
        Error::with_description("Invalid DOI or arXiv ID!", ErrorKind::InvalidValue).exit();
    }

    let entry = &input.entries()[0];

    let extensions = entry.extensions();

    if entry.authors().is_empty() || entry.published().is_none() || entry.id().len() == 0 {
        Error::with_description("Invalid DOI or arXiv ID!", ErrorKind::InvalidValue).exit();
    }

    assert!(extensions.contains_key("arxiv"));
    let arxiv_extension = extensions.get("arxiv").unwrap();
    if arxiv_extension.contains_key("doi") {
        let doi = arxiv_extension.get("doi").unwrap()[0].value().unwrap();
        let res = request_info(doi, IdType::Doi).await;
        return handle_response(res, IdType::Doi).await;
    }

    let mut firstauth = "".to_owned();

    let mut authors = "".to_owned();
    for (i, a) in entry.authors().iter().enumerate() {
        let name_vec = a.name().split_whitespace().collect::<Vec<_>>();
        assert!(!name_vec.is_empty());
        let n = name_vec.len();
        if i == 0 {
            firstauth = name_vec[n - 1].to_owned();
        }
        let ending = if i != entry.authors().len() - 1 {
            " and "
        } else {
            ""
        };
        authors.push_str(&format!(
            "{}, {}{}",
            name_vec[n - 1],
            name_vec[..n - 1].join(" "),
            ending
        ));
    }

    let categories = &entry.categories;
    assert!(!categories.is_empty());
    let class = categories[0].term();

    let year = entry.published().unwrap().year().to_string();
    let key = format!("{}_{}", firstauth, year);
    let title = format!("{}", &entry.title.as_str().replace("\n ", ""));
    let id = extract_id(&ARXIV_RE, entry.id());

    let formatted = format!(
        "@article{{{},title={{{}}},author={{{}}},year={{{}}},eprint={{{}}},archivePrefix={{arXiv}},primaryClass={{{}}}}}",
        key, title, authors, year, id, class
    );

    print_doi(&formatted)
}

#[async_recursion]
async fn handle_response(res: Result<Response, reqwest::Error>, idtype: IdType) -> String {
    if res.is_err() {
        Error::with_description("Invalid DOI or arXiv ID!", ErrorKind::InvalidValue).exit();
    }
    let res = res.unwrap().text_with_charset("utf-8").await.unwrap();
    if res.contains("cannot be found") {
        Error::with_description("Invalid DOI or arXiv ID!", ErrorKind::InvalidValue).exit();
    }
    match idtype {
        IdType::Doi => print_doi(&res),
        IdType::Arxiv => print_arxiv(&res.parse::<Feed>().unwrap()).await,
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
    pub static ref CLIENT: Client = Client::new();
}

#[derive(Debug, Clone, Copy)]
enum IdType {
    Doi,
    Arxiv,
}

async fn get_bibtex(pat: String) -> String {
    tokio::spawn(async move {
        let (id, idtype) =
            if DOI_IDENT_RE.is_match(&pat) || DOI_RE.iter().any(|re| re.is_match(&pat)) {
                (extract_id(&DOI_RE, &pat), IdType::Doi)
            } else if ARXIV_IDENT_RE.is_match(&pat) || ARXIV_RE.iter().any(|re| re.is_match(&pat)) {
                (extract_id(&ARXIV_RE, &pat), IdType::Arxiv)
            } else {
                Error::with_description(
                    "Please enter a valid DOI or arXiv ID!",
                    ErrorKind::InvalidValue,
                )
                .exit();
            };
        let res = request_info(&id, idtype).await;
        handle_response(res, idtype).await
    })
    .await
    .unwrap()
}

#[tokio::main]
async fn main() {
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

    let pats = if let Some(pats) = matches.values_of("input") {
        let mut pats = pats.collect::<Vec<_>>();
        pats.sort();
        pats.dedup();
        pats.into_iter()
            .map(|x| x.to_owned())
            .collect::<Vec<String>>()
    } else {
        Error::with_description("Missing arguments!", ErrorKind::MissingRequiredArgument).exit();
    };

    let mut futures = pats
        .into_iter()
        .map(|p| get_bibtex(p))
        .collect::<FuturesUnordered<_>>();

    while let Some(val) = futures.next().await {
        println!("{}", val);
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
