//! `lore-ingest` — Phase 0 Lore RAG 인덱싱 CLI.
//!
//! 사용법:
//!   cargo run --features embed --bin lore-ingest -- --all
//!   cargo run --features embed --bin lore-ingest -- --book jianghu-qixia-zh-1922
//!   cargo run --features embed --bin lore-ingest -- --all --reembed
//!
//! 환경변수:
//!   NPC_MIND_LORE_DB    SQLite 경로 (기본 data/corpus/lore.sqlite)
//!   NPC_MIND_MODEL_DIR  bge-m3 ONNX 모델 디렉토리 (기본 ../models/bge-m3)
//!   NPC_MIND_LORE_MANIFEST  매니페스트 경로 (기본 data/corpus/manifest.toml)
//!   NPC_MIND_LORE_BATCH 임베딩 배치 크기 (기본 32)

use std::path::PathBuf;
use std::process::ExitCode;

use npc_mind::adapter::ort_embedder::OrtEmbedder;
use npc_mind::lore::{
    EpubFileReader, LoreError, LoreStore, Manifest, SqliteLoreStore,
    ingest::{ingest_all, ingest_edition},
};

#[derive(Debug)]
struct Args {
    /// `--book <edition_id>` — 단일 edition만 ingest.
    book: Option<String>,
    /// `--all` — manifest 전체 순회.
    all: bool,
    /// `--reembed` — 이미 인덱싱된 edition도 재처리.
    reembed: bool,
    /// `--manifest <path>` 오버라이드.
    manifest_path: Option<PathBuf>,
    /// `--db <path>` 오버라이드.
    db_path: Option<PathBuf>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args { book: None, all: false, reembed: false, manifest_path: None, db_path: None };
    let mut iter = std::env::args().skip(1);
    while let Some(a) = iter.next() {
        match a.as_str() {
            "--all" => args.all = true,
            "--reembed" => args.reembed = true,
            "--book" => {
                args.book = Some(iter.next().ok_or("--book requires a value")?);
            }
            "--manifest" => {
                args.manifest_path = Some(iter.next().ok_or("--manifest requires a value")?.into());
            }
            "--db" => {
                args.db_path = Some(iter.next().ok_or("--db requires a value")?.into());
            }
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            other => return Err(format!("Unknown argument: {other}")),
        }
    }
    if !args.all && args.book.is_none() {
        return Err("Either --all or --book <edition_id> is required".into());
    }
    Ok(args)
}

fn print_help() {
    println!(
        "lore-ingest — Phase 0 Lore RAG 인덱싱\n\n\
        USAGE:\n\
        \tcargo run --features embed --bin lore-ingest -- [OPTIONS]\n\n\
        OPTIONS:\n\
        \t--all                  manifest 전체 ingest\n\
        \t--book <edition_id>    한 edition만 ingest\n\
        \t--reembed              이미 인덱싱된 edition도 재처리\n\
        \t--manifest <path>      매니페스트 경로 오버라이드\n\
        \t--db <path>            SQLite 경로 오버라이드"
    );
}

fn main() -> ExitCode {
    let _ = tracing_subscriber_init();
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            print_help();
            return ExitCode::from(2);
        }
    };

    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ingest 실패: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Args) -> Result<(), LoreError> {
    let manifest_path = args
        .manifest_path
        .or_else(|| std::env::var_os("NPC_MIND_LORE_MANIFEST").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("data/corpus/manifest.toml"));

    let db_path = args
        .db_path
        .or_else(|| std::env::var_os("NPC_MIND_LORE_DB").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("data/corpus/lore.sqlite"));

    let model_dir = std::env::var("NPC_MIND_MODEL_DIR")
        .unwrap_or_else(|_| "../models/bge-m3".to_string());
    let model_path = std::path::Path::new(&model_dir).join("model_quantized.onnx");
    let tokenizer_path = std::path::Path::new(&model_dir).join("tokenizer.json");

    let batch_size: usize = std::env::var("NPC_MIND_LORE_BATCH")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(32);

    println!("[lore-ingest] manifest={}", manifest_path.display());
    println!("[lore-ingest] db={}", db_path.display());
    println!("[lore-ingest] model={}", model_path.display());

    let manifest = Manifest::load(&manifest_path)
        .map_err(|e| LoreError::Storage(format!("manifest 로드: {e}")))?;

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let store = SqliteLoreStore::new(
        db_path.to_str().ok_or_else(|| LoreError::Storage("db 경로 UTF-8 변환 실패".into()))?,
    )?;

    let mut embedder = OrtEmbedder::new(&model_path, &tokenizer_path)
        .map_err(|e| LoreError::Storage(format!("OrtEmbedder 초기화: {e:?}")))?;

    let mut reader = EpubFileReader::new();

    if args.all {
        let stats = ingest_all(
            &mut reader,
            &mut embedder,
            &store,
            &manifest,
            !args.reembed,
            batch_size,
            |edition_id, status| {
                println!("[lore-ingest] {edition_id}: {status}");
            },
        )?;
        println!("\n=== 결과 ===");
        for s in &stats {
            println!(
                "{:30} chapters={:5} chunks={:7}",
                s.edition_id, s.chapter_count, s.chunk_count
            );
        }
    } else if let Some(book) = args.book.as_deref() {
        let (corpus, edition) = manifest
            .find_edition(book)
            .ok_or_else(|| LoreError::Storage(format!("edition '{book}' 없음")))?;
        if !args.reembed {
            let n = store.count_chunks(&edition.id).unwrap_or(0);
            if n > 0 {
                println!("[lore-ingest] {book}: 이미 {n}개 청크 인덱싱됨 — --reembed 미지정으로 skip");
                return Ok(());
            }
        }
        println!("[lore-ingest] {book}: starting...");
        let stats = ingest_edition(
            &mut reader,
            &mut embedder,
            &store,
            corpus,
            edition,
            batch_size,
            |done, total| {
                if done % 100 == 0 || done == total {
                    println!("  {done}/{total}");
                }
            },
        )?;
        println!(
            "\n{:30} chapters={:5} chunks={:7}",
            stats.edition_id, stats.chapter_count, stats.chunk_count
        );
    }

    Ok(())
}

fn tracing_subscriber_init() -> Result<(), Box<dyn std::error::Error>> {
    // 라이브러리에서 tracing이 활성이지만 별도 subscriber 없이도 stdout println으로 충분.
    // 향후 확장 위해 빈 함수로 둠.
    Ok(())
}
