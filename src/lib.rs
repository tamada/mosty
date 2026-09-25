use std::path::{Path, PathBuf};

use calamine::{Reader, Xlsx};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{}", render_group(.0))]
    Array(Vec<Self>),
    #[error("{0}")]
    Calamine(#[source] calamine::Error),
    #[error("{0}")]
    Xlsx(#[source] calamine::XlsxError),
    #[error("{0}")]
    Clap(#[source] clap::Error),
    #[error("IO error for {path}: {cause}", path = .0.display(), cause = .1)]
    Io(PathBuf, #[source] std::io::Error),
}

impl Error {
    pub fn vec_result_to_result_vec<T>(from: Vec<Result<T>>) -> Result<Vec<T>> {
        let mut results = Vec::new();
        let mut errs = Vec::new();
        for r in from {
            match r {
                Ok(v) => results.push(v),
                Err(e) => errs.push(e),
            }
        }
        Self::error_or(results, errs)
    }

    pub fn error_or<T>(result: T, errs: Vec<Self>) -> Result<T> {
        if errs.is_empty() {
            Ok(result)
        } else if errs.len() == 1 {
            Err(errs.into_iter().next().unwrap())
        } else {
            Err(Error::Array(errs))
        }
    }
}

fn render_group(errs: &[Error]) -> String {
    let mut s = String::from("Multiple errors:");
    errs.iter()
        .enumerate()
        .for_each(|(i, err)| s.push_str(&format!("\n {:>3}.  {}", i + 1, err)));
    s
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn verify(path: &Path) -> Result<String> {
    let mut excel: Xlsx<_> = calamine::open_workbook(path).map_err(Error::Xlsx)?;
    let worksheets = excel.worksheets();
    for (name, data) in worksheets {}

    Ok("ok".to_string())
}
