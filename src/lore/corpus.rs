//! Lore corpus 매니페스트 — `data/corpus/manifest.toml` 파싱.
//!
//! Phase 0: PD 원전 3권 등록. 확장 시 디렉터 승인 필수.
//! 매니페스트는 원본 EPUB 파일 경로(gitignore)와 SQLite 인덱스(gitignore)를 잇는
//! 단일 진입점이다. 다른 머신에서 `lore-ingest --all` 호출 시 이 파일이 source-of-truth.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// 매니페스트 루트 — `[[corpus]]` 배열을 가진 TOML 문서.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub corpus: Vec<CorpusMeta>,
}

/// 한 작품(원전) 단위. 작가·장르·라이선스 + N개 edition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusMeta {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub author_name: Option<String>,
    #[serde(default)]
    pub genre_tags: Vec<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub license_note: Option<String>,
    #[serde(default)]
    pub editions: Vec<Edition>,
}

/// 한 작품의 특정 판본 — 같은 작품의 다른 번역/주석/언어를 구분.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edition {
    pub id: String,
    pub language: String,
    #[serde(default)]
    pub edition: Option<String>,
    pub source: String,
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default)]
    pub license_note: Option<String>,
}

fn default_format() -> String {
    "epub".to_string()
}

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("매니페스트 파일을 읽을 수 없음: {0}")]
    Io(#[from] std::io::Error),
    #[error("매니페스트 파싱 실패: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("edition '{0}'을 찾을 수 없음")]
    EditionNotFound(String),
    #[error("corpus '{0}'을 찾을 수 없음")]
    CorpusNotFound(String),
}

impl Manifest {
    /// 파일에서 매니페스트 로드.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ManifestError> {
        let raw = std::fs::read_to_string(path)?;
        Self::from_toml(&raw)
    }

    /// TOML 문자열에서 직접 파싱 (테스트·메모리 입력용).
    pub fn from_toml(raw: &str) -> Result<Self, ManifestError> {
        Ok(toml::from_str(raw)?)
    }

    /// edition_id로 (corpus, edition) 쌍을 찾는다.
    pub fn find_edition(&self, edition_id: &str) -> Option<(&CorpusMeta, &Edition)> {
        for c in &self.corpus {
            for e in &c.editions {
                if e.id == edition_id {
                    return Some((c, e));
                }
            }
        }
        None
    }

    /// corpus_id로 corpus를 찾는다.
    pub fn find_corpus(&self, corpus_id: &str) -> Option<&CorpusMeta> {
        self.corpus.iter().find(|c| c.id == corpus_id)
    }

    /// 모든 (corpus, edition) 쌍을 평탄화 — `--all` ingest 순회용.
    pub fn iter_editions(&self) -> impl Iterator<Item = (&CorpusMeta, &Edition)> {
        self.corpus
            .iter()
            .flat_map(|c| c.editions.iter().map(move |e| (c, e)))
    }
}

impl Edition {
    /// `source`를 절대/상대 경로로 해석. 매니페스트 디렉토리 기준이 아닌
    /// 프로세스 cwd 기준 — Mind Studio와 ingest CLI 모두 repo 루트에서 실행한다는 가정.
    pub fn source_path(&self) -> PathBuf {
        PathBuf::from(&self.source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PHASE0_MANIFEST: &str = include_str!("../../data/corpus/manifest.toml");

    #[test]
    fn manifest_parses() {
        let manifest = Manifest::from_toml(PHASE0_MANIFEST).expect("manifest.toml 파싱 실패");
        assert_eq!(manifest.corpus.len(), 3, "Phase 0 = PD 원전 3권");

        // 각 corpus가 최소 1 edition을 가져야 함
        for c in &manifest.corpus {
            assert!(
                !c.editions.is_empty(),
                "corpus '{}'에 edition이 없음",
                c.id
            );
            assert_eq!(c.license.as_deref(), Some("public-domain"));
        }

        // 3권의 edition_id 존재 검증
        for eid in [
            "shuihuzhuan-zh-zhang",
            "jianghu-qixia-zh-1922",
            "shushan-jianxia-zh-1932",
        ] {
            assert!(
                manifest.find_edition(eid).is_some(),
                "edition '{eid}' 누락"
            );
        }
    }

    #[test]
    fn iter_editions_visits_all() {
        let manifest = Manifest::from_toml(PHASE0_MANIFEST).unwrap();
        let count = manifest.iter_editions().count();
        assert_eq!(count, 3);
    }

    #[test]
    fn find_corpus_returns_metadata() {
        let manifest = Manifest::from_toml(PHASE0_MANIFEST).unwrap();
        let c = manifest.find_corpus("jianghu-qixia-zhuan").unwrap();
        assert!(c.genre_tags.contains(&"wuxia".to_string()));
    }
}
