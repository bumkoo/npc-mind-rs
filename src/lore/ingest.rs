//! Ingest 파이프라인 — EPUB → 챕터 추출 → 청킹 → 임베딩 → `LoreStore`.
//!
//! Phase 0: 중국어 EPUB 3권만 대상. 챕터 경계는 청크 경계로 보존(같은 청크 안에 두
//! 챕터를 섞지 않음). 한국어/영어 청크 사이즈는 §6.3 결정 사항에 따라 별도 상수.
//!
//! 모든 함수는 `embed` feature 안에서만 의미가 있으나 (실제 EPUB 파싱 + 임베딩 요구),
//! 청킹·텍스트 정제 함수는 feature 없이도 컴파일·테스트 가능.

use super::corpus::Edition;
#[cfg(feature = "embed")]
use super::corpus::{CorpusMeta, Manifest};
use super::store::{ChunkRecord, LoreError};
#[cfg(feature = "embed")]
use super::store::LoreStore;

/// 언어별 청킹 파라미터 — §6.3 결정 사항.
#[derive(Debug, Clone, Copy)]
pub struct ChunkConfig {
    /// 한 청크의 목표 글자 수 (Unicode scalar count, EPUB은 한자 단위로 처리).
    pub target_chars: usize,
    /// 인접 청크 간 overlap 글자 수.
    pub overlap_chars: usize,
}

impl ChunkConfig {
    /// 언어 코드별 기본값.
    /// `zh`, `zh-Hans`, `zh-Hant`: 500/200
    /// `ko`: 1000/200, `en`: 1500/300, 그 외: zh와 동일.
    pub fn for_language(lang: &str) -> Self {
        let l = lang.to_lowercase();
        if l.starts_with("zh") {
            Self { target_chars: 500, overlap_chars: 200 }
        } else if l.starts_with("ko") {
            Self { target_chars: 1000, overlap_chars: 200 }
        } else if l.starts_with("en") {
            Self { target_chars: 1500, overlap_chars: 300 }
        } else {
            Self { target_chars: 500, overlap_chars: 200 }
        }
    }
}

/// 파싱 단계의 챕터 단위 — 본문은 normalize 완료된 plain text.
#[derive(Debug, Clone)]
pub struct ChapterText {
    pub index: u32,
    pub title: Option<String>,
    pub text: String,
}

/// 청크 최소 길이 (Unicode scalar) — 이 미만은 noise(짧은 ToC 항목, 빈 페이지 잔여 등)로 간주하고 skip.
pub const MIN_CHUNK_CHARS: usize = 50;

/// ToC·표지 같은 비본문 챕터 제목 화이트리스트.
/// 정확 일치 또는 prefix 매칭(목차 1./Contents — 등)으로 skip.
const NOISE_CHAPTER_TITLES: &[&str] = &[
    "Cover",
    "封面",
    "目錄",
    "目次",
    "目录",
    "Table of Contents",
    "Contents",
];

/// 챕터 제목이 ToC/표지 등 비본문이면 true.
/// 비교는 trim + 정확 일치 (대소문자 영어는 그대로 비교 — 등재된 형태로만 매칭).
pub fn is_noise_chapter_title(title: &str) -> bool {
    let t = title.trim();
    if t.is_empty() {
        return false;
    }
    NOISE_CHAPTER_TITLES.contains(&t)
}

/// EPUB → 챕터 텍스트 추출 트레잇. Phase 0의 단일 구현은 `epub` 크레이트 기반.
pub trait EpubReader {
    fn read_chapters(&mut self, path: &std::path::Path) -> Result<Vec<ChapterText>, LoreError>;
}

/// 챕터 1개를 char 기준으로 슬라이딩 윈도우 청킹.
///
/// 챕터 경계 밖으로 overlap이 새지 않도록 챕터 단위로 호출한다.
/// 결과 청크의 `char_offset_in_chapter`는 시작 위치, `char_offset_in_edition`은
/// `edition_offset_base + char_offset_in_chapter`.
pub fn chunk_chapter(
    corpus_id: &str,
    edition_id: &str,
    language: &str,
    chapter: &ChapterText,
    edition_offset_base: u64,
    cfg: ChunkConfig,
) -> Vec<ChunkRecord> {
    let chars: Vec<char> = chapter.text.chars().collect();
    if chars.is_empty() {
        return Vec::new();
    }
    let target = cfg.target_chars.max(1);
    let overlap = cfg.overlap_chars.min(target.saturating_sub(1));
    let step = target.saturating_sub(overlap).max(1);

    let mut out = Vec::new();
    let mut start = 0usize;
    let mut idx = 0usize;
    while start < chars.len() {
        let end = (start + target).min(chars.len());
        let len = end - start;
        // 노이즈 필터: 청크 길이 < MIN_CHUNK_CHARS면 skip.
        // 마지막 짧은 꼬리(전 청크의 overlap에 이미 포함)와 빈 페이지 잔여를 함께 처리.
        if len >= MIN_CHUNK_CHARS {
            let text: String = chars[start..end].iter().collect();
            let chunk_id = format!(
                "{edition_id}::ch{:04}::p{:04}",
                chapter.index,
                idx
            );
            out.push(ChunkRecord {
                chunk_id,
                corpus_id: corpus_id.to_string(),
                edition_id: edition_id.to_string(),
                language: language.to_string(),
                text,
                chapter_index: Some(chapter.index),
                chapter_title: chapter.title.clone(),
                char_offset_in_edition: edition_offset_base + start as u64,
                char_offset_in_chapter: start as u64,
            });
            idx += 1;
        }
        if end >= chars.len() {
            break;
        }
        start += step;
    }
    out
}

/// 한 edition 전체를 chunked records로 변환.
///
/// 노이즈 필터:
/// - `chapter_title`이 ToC/표지 등 비본문 제목이면 챕터 전체 skip
///   (단 `char_offset_in_edition` 누적은 그대로 유지 → 본문 챕터 offset 일관성 보존)
/// - 개별 청크 길이 < `MIN_CHUNK_CHARS`는 `chunk_chapter` 내부에서 skip
pub fn chunk_edition(
    corpus_id: &str,
    edition: &Edition,
    chapters: &[ChapterText],
) -> Vec<ChunkRecord> {
    let cfg = ChunkConfig::for_language(&edition.language);
    let mut out = Vec::new();
    let mut base: u64 = 0;
    for ch in chapters {
        let chapter_chars = ch.text.chars().count() as u64;
        let skip = ch
            .title
            .as_deref()
            .map(is_noise_chapter_title)
            .unwrap_or(false);
        if !skip {
            let mut recs =
                chunk_chapter(corpus_id, &edition.id, &edition.language, ch, base, cfg);
            out.append(&mut recs);
        }
        base += chapter_chars;
    }
    out
}

/// Ingest 한 edition: EPUB 읽기 → 청킹 → batch 임베딩 → 저장.
///
/// 단일 트랜잭션으로 처리하지 않고 batch_size 단위로 분할 commit (긴 EPUB의 메모리
/// 폭주 방지). 진행률은 `progress` 콜백으로 호출자가 로그 출력.
#[cfg(feature = "embed")]
pub fn ingest_edition(
    reader: &mut dyn EpubReader,
    embedder: &mut dyn crate::ports::TextEmbedder,
    store: &dyn LoreStore,
    corpus: &CorpusMeta,
    edition: &Edition,
    batch_size: usize,
    mut progress: impl FnMut(usize, usize),
) -> Result<IngestStats, LoreError> {
    let path = edition.source_path();
    let chapters = reader.read_chapters(&path)?;
    let chunks = chunk_edition(&corpus.id, edition, &chapters);
    let total = chunks.len();
    if total == 0 {
        return Ok(IngestStats {
            edition_id: edition.id.clone(),
            chapter_count: chapters.len(),
            chunk_count: 0,
        });
    }

    let bs = batch_size.max(1);
    let mut written = 0usize;
    for batch in chunks.chunks(bs) {
        let texts: Vec<&str> = batch.iter().map(|c| c.text.as_str()).collect();
        let embs = embedder
            .embed(&texts)
            .map_err(|e| LoreError::Storage(format!("embed: {e:?}")))?;
        store.upsert_batch(batch, &embs)?;
        written += batch.len();
        progress(written, total);
    }

    Ok(IngestStats {
        edition_id: edition.id.clone(),
        chapter_count: chapters.len(),
        chunk_count: total,
    })
}

/// `--all`: manifest 전체를 순회하며 ingest. `skip_existing=true`면 이미 인덱싱된
/// edition은 건너뜀. (count_chunks > 0)
#[cfg(feature = "embed")]
pub fn ingest_all(
    reader: &mut dyn EpubReader,
    embedder: &mut dyn crate::ports::TextEmbedder,
    store: &dyn LoreStore,
    manifest: &Manifest,
    skip_existing: bool,
    batch_size: usize,
    mut on_edition: impl FnMut(&str, &str),
) -> Result<Vec<IngestStats>, LoreError> {
    let mut out = Vec::new();
    for (corpus, edition) in manifest.iter_editions() {
        if skip_existing {
            let n = store.count_chunks(&edition.id).unwrap_or(0);
            if n > 0 {
                on_edition(&edition.id, "skipped");
                continue;
            }
        }
        on_edition(&edition.id, "starting");
        let stats = ingest_edition(reader, embedder, store, corpus, edition, batch_size, |_, _| {})?;
        on_edition(&edition.id, "done");
        out.push(stats);
    }
    Ok(out)
}

#[cfg(not(feature = "embed"))]
#[allow(dead_code)]
pub fn ingest_edition_disabled() -> Result<(), LoreError> {
    Err(LoreError::FeatureDisabled("ingest는 --features embed 필요"))
}

/// Ingest 결과 — 보고서 출력용.
#[derive(Debug, Clone)]
pub struct IngestStats {
    pub edition_id: String,
    pub chapter_count: usize,
    pub chunk_count: usize,
}

// ---------------------------------------------------------------------------
// EPUB 어댑터 (embed feature) — `epub` 크레이트 기반.
// ---------------------------------------------------------------------------

#[cfg(feature = "embed")]
pub use epub_impl::EpubFileReader;

#[cfg(feature = "embed")]
mod epub_impl {
    use super::*;
    use std::path::Path;

    /// `epub` 크레이트 (https://crates.io/crates/epub) 기반 EpubReader 구현.
    ///
    /// EPUB의 spine 순서대로 XHTML 문서를 읽고, 간이 HTML→텍스트 변환을 적용한 뒤
    /// 챕터 단위로 반환한다. 챕터 제목은 ToC entry 또는 첫 `<h*>` 태그에서 추출.
    pub struct EpubFileReader;

    impl EpubFileReader {
        pub fn new() -> Self { Self }
    }

    impl Default for EpubFileReader {
        fn default() -> Self { Self::new() }
    }

    impl EpubReader for EpubFileReader {
        fn read_chapters(&mut self, path: &Path) -> Result<Vec<ChapterText>, LoreError> {
            let mut doc = epub::doc::EpubDoc::new(path)
                .map_err(|e| LoreError::Storage(format!("EPUB 열기 실패 {}: {e}", path.display())))?;

            // ToC 매핑: spine resource path → 제목
            let toc_titles: std::collections::HashMap<String, String> = doc
                .toc
                .iter()
                .map(|n| (n.content.to_string_lossy().into_owned(), n.label.clone()))
                .collect();

            let spine_len = doc.spine.len();
            let mut chapters = Vec::with_capacity(spine_len);

            for i in 0..spine_len {
                let _ = doc.set_current_chapter(i);
                let res_id = doc.spine[i].idref.clone();
                let res_path = doc
                    .resources
                    .get(&res_id)
                    .map(|item| item.path.to_string_lossy().into_owned())
                    .unwrap_or_default();

                let raw = match doc.get_current_str() {
                    Some((s, _mime)) => s,
                    None => continue,
                };
                let text = html_to_text(&raw);
                if text.trim().is_empty() {
                    continue;
                }

                let title = toc_titles
                    .get(&res_path)
                    .cloned()
                    .or_else(|| extract_first_heading(&raw));

                chapters.push(ChapterText {
                    index: chapters.len() as u32 + 1,
                    title,
                    text,
                });
            }

            Ok(chapters)
        }
    }

    /// 단순 HTML → plain text. 파싱 라이브러리 없이 태그를 벗기고 엔티티 디코드.
    /// EPUB 본문은 일반적으로 well-formed XHTML이라 정규식 없이 상태 머신으로 충분.
    fn html_to_text(html: &str) -> String {
        let mut out = String::with_capacity(html.len());
        let mut in_tag = false;
        let mut in_script_or_style = false;
        let mut block_break_pending = false;
        let bytes = html.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if !in_tag && b == b'<' {
                // 태그 시작 — 토큰 보고 script/style/block break 결정
                let tag_end = bytes[i..].iter().position(|c| *c == b'>').map(|p| i + p);
                let tag = match tag_end {
                    Some(end) => &html[i + 1..end],
                    None => &html[i + 1..],
                };
                let lower = tag.to_ascii_lowercase();
                let tag_name = lower
                    .trim_start_matches('/')
                    .split_whitespace()
                    .next()
                    .unwrap_or("");
                // block-level 또는 br 태그면 줄바꿈
                if matches!(
                    tag_name,
                    "p" | "br" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "li" | "tr"
                ) {
                    block_break_pending = true;
                }
                if tag_name == "script" || tag_name == "style" {
                    in_script_or_style = !lower.starts_with('/');
                }
                in_tag = true;
                i += 1;
                continue;
            }
            if in_tag {
                if b == b'>' {
                    in_tag = false;
                }
                i += 1;
                continue;
            }
            if in_script_or_style {
                i += 1;
                continue;
            }
            if block_break_pending {
                if !out.ends_with('\n') {
                    out.push('\n');
                }
                block_break_pending = false;
            }
            // 엔티티 디코드 (간이 — 자주 보이는 것만)
            if b == b'&'
                && let Some(semi) = html[i..].find(';') {
                    let ent = &html[i + 1..i + semi];
                    let decoded = decode_entity(ent);
                    out.push_str(&decoded);
                    i += semi + 1;
                    continue;
                }

            // UTF-8 codepoint 통째로 push
            let ch_len = utf8_char_len(b);
            let end = (i + ch_len).min(bytes.len());
            out.push_str(&html[i..end]);
            i = end;
        }
        // 다중 공백·줄바꿈 정리
        let mut cleaned = String::with_capacity(out.len());
        let mut prev_blank = false;
        for line in out.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                if !prev_blank && !cleaned.is_empty() {
                    cleaned.push('\n');
                    prev_blank = true;
                }
            } else {
                cleaned.push_str(trimmed);
                cleaned.push('\n');
                prev_blank = false;
            }
        }
        cleaned
    }

    fn utf8_char_len(b: u8) -> usize {
        if b < 0xC0 { 1 } // standard ASCII or continuation byte (fallback)
        else if b < 0xE0 { 2 }
        else if b < 0xF0 { 3 }
        else { 4 }
    }

    fn decode_entity(ent: &str) -> String {
        match ent {
            "amp" => "&".into(),
            "lt" => "<".into(),
            "gt" => ">".into(),
            "quot" => "\"".into(),
            "apos" => "'".into(),
            "nbsp" => " ".into(),
            s if s.starts_with("#x") || s.starts_with("#X") => {
                u32::from_str_radix(&s[2..], 16)
                    .ok()
                    .and_then(char::from_u32)
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| format!("&{ent};"))
            }
            s if s.starts_with('#') => {
                s[1..]
                    .parse::<u32>()
                    .ok()
                    .and_then(char::from_u32)
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| format!("&{ent};"))
            }
            _ => format!("&{ent};"),
        }
    }

    /// `<h1>...</h1>` ~ `<h6>...</h6>` 중 첫 등장 텍스트.
    fn extract_first_heading(html: &str) -> Option<String> {
        for tag in ["h1", "h2", "h3", "h4", "h5", "h6"] {
            let open = format!("<{tag}");
            if let Some(start) = html.to_ascii_lowercase().find(&open) {
                let after_open = html[start..].find('>').map(|p| start + p + 1)?;
                let close = format!("</{tag}");
                let close_pos = html[after_open..].to_ascii_lowercase().find(&close)?;
                let inner = &html[after_open..after_open + close_pos];
                let text = html_to_text(inner).trim().to_string();
                if !text.is_empty() {
                    return Some(text);
                }
            }
        }
        None
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn html_to_text_strips_tags() {
            let html = "<html><body><h1>第一回</h1><p>江湖之中</p><p>義氣為先</p></body></html>";
            let t = html_to_text(html);
            assert!(t.contains("第一回"));
            assert!(t.contains("江湖之中"));
            assert!(t.contains("義氣為先"));
        }

        #[test]
        fn html_to_text_decodes_entities() {
            let t = html_to_text("<p>A &amp; B &#x4E00; &nbsp;C</p>");
            assert!(t.contains('&'));
            assert!(t.contains('一'));
        }

        #[test]
        fn extract_first_heading_works() {
            let h = extract_first_heading("<div><h2>第二回 章節</h2><p>x</p></div>");
            assert_eq!(h.as_deref(), Some("第二回 章節"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_chapter(idx: u32, title: &str, n: usize) -> ChapterText {
        ChapterText {
            index: idx,
            title: Some(title.into()),
            text: "天".repeat(n), // 한자 1글자 반복
        }
    }

    #[test]
    fn chunk_config_per_language() {
        assert_eq!(ChunkConfig::for_language("zh").target_chars, 500);
        assert_eq!(ChunkConfig::for_language("zh-Hant").target_chars, 500);
        assert_eq!(ChunkConfig::for_language("ko").target_chars, 1000);
        assert_eq!(ChunkConfig::for_language("en").target_chars, 1500);
    }

    #[test]
    fn chunks_respect_chapter_boundaries() {
        let cfg = ChunkConfig { target_chars: 100, overlap_chars: 20 };
        let ch = fake_chapter(1, "ch1", 250);
        let recs = chunk_chapter("c", "e", "zh", &ch, 0, cfg);
        // 250 chars, target 100, step 80 → starts at 0, 80, 160, 240
        assert!(recs.len() >= 3);
        // 모두 chapter 1
        assert!(recs.iter().all(|r| r.chapter_index == Some(1)));
        // 마지막 청크가 250자를 넘어서지 않음
        for r in &recs {
            assert!(r.text.chars().count() <= 100);
        }
    }

    #[test]
    fn edition_offsets_increment_across_chapters() {
        let edition = Edition {
            id: "ed".into(),
            language: "zh".into(),
            edition: None,
            source: "x".into(),
            format: "epub".into(),
            license_note: None,
        };
        // 두 챕터 모두 MIN_CHUNK_CHARS(50) 이상이어야 noise 필터에 걸리지 않음.
        let chapters = vec![fake_chapter(1, "a", 100), fake_chapter(2, "b", 80)];
        let recs = chunk_edition("c", &edition, &chapters);
        assert!(!recs.is_empty());
        let chapter1: Vec<_> = recs.iter().filter(|r| r.chapter_index == Some(1)).collect();
        let chapter2: Vec<_> = recs.iter().filter(|r| r.chapter_index == Some(2)).collect();
        assert!(!chapter2.is_empty());
        // chapter2의 첫 청크는 chapter1의 끝(100자) 이상의 edition offset
        let c2_first = chapter2[0].char_offset_in_edition;
        let c1_first = chapter1[0].char_offset_in_edition;
        assert!(c2_first >= 100);
        assert!(c1_first < c2_first);
        // chapter 안에서 청크는 같은 챕터로만 채워짐
        for r in &chapter2 {
            assert_eq!(r.chapter_index, Some(2));
        }
    }

    /// Cleanup TASK: ToC/표지 챕터 + 짧은 청크 noise 필터 단위 테스트.
    /// `chunk_edition` 진입 시 noise 챕터는 통째로 skip되고, 본문 챕터의 짧은 꼬리는
    /// `chunk_chapter` 내부에서 skip되며, 본문 챕터의 `char_offset_in_edition`은
    /// (skip된 noise 챕터 길이만큼) 그대로 누적된다.
    #[test]
    fn noise_filter_skips_toc_and_short_chunks() {
        // is_noise_chapter_title 단위 검증
        assert!(is_noise_chapter_title("Cover"));
        assert!(is_noise_chapter_title("封面"));
        assert!(is_noise_chapter_title("目錄"));
        assert!(is_noise_chapter_title("目次"));
        assert!(is_noise_chapter_title("目录"));
        assert!(is_noise_chapter_title("Table of Contents"));
        assert!(is_noise_chapter_title("Contents"));
        assert!(is_noise_chapter_title("  Cover  "));     // trim
        assert!(!is_noise_chapter_title("第一回"));
        assert!(!is_noise_chapter_title(""));

        let edition = Edition {
            id: "ed".into(),
            language: "zh".into(),
            edition: None,
            source: "x".into(),
            format: "epub".into(),
            license_note: None,
        };

        // 챕터 1: ToC(목차) — 통째로 skip
        // 챕터 2: 본문 100자 — 살아남음
        // 챕터 3: 본문 30자(< MIN_CHUNK_CHARS) — chunk_chapter에서 모든 청크 skip
        // 챕터 4: 본문 200자 — 살아남음
        let chapters = vec![
            fake_chapter(1, "目錄", 40),
            fake_chapter(2, "第一回", 100),
            fake_chapter(3, "短章", 30),
            fake_chapter(4, "第二回", 200),
        ];
        let recs = chunk_edition("c", &edition, &chapters);

        // 살아남는 챕터는 2와 4뿐
        let kept_chapters: std::collections::BTreeSet<u32> =
            recs.iter().filter_map(|r| r.chapter_index).collect();
        assert_eq!(kept_chapters, [2u32, 4].into_iter().collect());

        // ToC(40자)와 짧은 챕터(30자)에서는 청크가 0개
        assert!(!recs.iter().any(|r| r.chapter_index == Some(1)));
        assert!(!recs.iter().any(|r| r.chapter_index == Some(3)));

        // 본문 챕터는 char_offset_in_edition이 누적 길이 기준으로 매겨짐:
        // 챕터 2의 첫 청크 offset = 40 (목차 길이)
        // 챕터 4의 첫 청크 offset = 40 + 100 + 30 = 170
        let ch2_first = recs.iter().find(|r| r.chapter_index == Some(2)).unwrap();
        let ch4_first = recs.iter().find(|r| r.chapter_index == Some(4)).unwrap();
        assert_eq!(ch2_first.char_offset_in_edition, 40);
        assert_eq!(ch4_first.char_offset_in_edition, 170);

        // 모든 살아남은 청크는 MIN_CHUNK_CHARS 이상
        for r in &recs {
            assert!(
                r.text.chars().count() >= MIN_CHUNK_CHARS,
                "노이즈 필터를 통과한 청크가 {}자 — MIN_CHUNK_CHARS({}) 이상이어야 함",
                r.text.chars().count(),
                MIN_CHUNK_CHARS
            );
        }
    }
}
