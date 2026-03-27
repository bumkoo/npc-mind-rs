//! DebugApp — eframe 앱 구현

use eframe::egui;

use npc_mind::domain::emotion::EmotionState;

use crate::panels;
use crate::pipeline::{self, PipelineResult};
use crate::state::GuiState;

pub struct DebugApp {
    pub state: GuiState,
    pub results: Vec<PipelineResult>,
    /// 현재 감정 상태 (자극 적용 시 체이닝용)
    pub current_emotion: Option<EmotionState>,
    /// PadAnalyzer (embed feature 활성 시에만 로드)
    #[cfg(feature = "embed")]
    pub pad_analyzer: Option<npc_mind::domain::pad::PadAnalyzer>,
    #[cfg(feature = "embed")]
    pub embed_status: String,
}

impl DebugApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        #[cfg(feature = "embed")]
        let (pad_analyzer, embed_status) = {
            match load_pad_analyzer() {
                Ok(analyzer) => (Some(analyzer), "임베딩 모델 로드 완료".to_string()),
                Err(e) => (None, format!("임베딩 모델 로드 실패: {e}")),
            }
        };

        Self {
            state: GuiState::default(),
            results: Vec::new(),
            current_emotion: None,
            #[cfg(feature = "embed")]
            pad_analyzer,
            #[cfg(feature = "embed")]
            embed_status,
        }
    }
}

#[cfg(feature = "embed")]
fn load_pad_analyzer() -> Result<npc_mind::domain::pad::PadAnalyzer, String> {
    use npc_mind::adapter::ort_embedder::OrtEmbedder;

    let model_dir = std::path::Path::new("../models/bge-m3");
    let model_path = model_dir.join("model_quantized.onnx");
    let tokenizer_path = model_dir.join("tokenizer.json");

    if !model_path.exists() {
        return Err(format!("모델 파일 없음: {}", model_path.display()));
    }

    let embedder = OrtEmbedder::new(&model_path, &tokenizer_path)
        .map_err(|e| format!("OrtEmbedder 초기화 실패: {e}"))?;

    npc_mind::domain::pad::PadAnalyzer::new(Box::new(embedder))
        .map_err(|e| format!("PadAnalyzer 초기화 실패: {e}"))
}

impl eframe::App for DebugApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 상단 입력 패널
        egui::TopBottomPanel::top("input_panel")
            .resizable(true)
            .default_height(420.0)
            .show(ctx, |ui| {
                panels::input_panel::show(ui, &mut self.state);
            });

        // 버튼 바 — 입력과 출력 사이 고정
        egui::TopBottomPanel::top("action_bar")
            .exact_height(36.0)
            .show(ctx, |ui| {
                show_action_buttons(ui, self);
            });

        // 하단 결과 패널
        egui::CentralPanel::default().show(ctx, |ui| {
            panels::output_panel::show(ui, &mut self.results);
        });
    }
}

fn show_action_buttons(ui: &mut egui::Ui, app: &mut DebugApp) {
    ui.horizontal(|ui| {
        let has_emotion = app.current_emotion.is_some();

        if ui.button("감정 평가").clicked() {
            let (emotion, result) = pipeline::run_appraise(&app.state);
            app.current_emotion = Some(emotion);
            app.results.push(result);
        }
        if ui.button("가이드 생성").clicked() {
            let (emotion, result) = pipeline::run_guide(&app.state);
            app.current_emotion = Some(emotion);
            app.results.push(result);
        }

        ui.separator();

        // PAD 평가 버튼 (embed feature)
        show_pad_evaluate_button(ui, app);

        let stimulus_btn = egui::Button::new("자극 적용");
        if ui.add_enabled(has_emotion, stimulus_btn).clicked() {
            if let Some(ref current) = app.current_emotion.clone() {
                let (new_emotion, result) = pipeline::run_stimulus(&app.state, current);
                app.current_emotion = Some(new_emotion);
                app.results.push(result);
            }
        }

        ui.separator();

        let end_btn = egui::Button::new("대화 종료");
        if ui.add_enabled(has_emotion, end_btn).clicked() {
            if let Some(ref current) = app.current_emotion.clone() {
                let (new_rel, result) = pipeline::run_after_dialogue(&app.state, current);
                app.state.closeness = new_rel.closeness().value();
                app.state.trust = new_rel.trust().value();
                app.state.power = new_rel.power().value();
                app.results.push(result);
            }
        }

        if !has_emotion {
            ui.label(
                egui::RichText::new("  (먼저 감정 평가를 실행하세요)")
                    .weak()
                    .italics(),
            );
        }
    });
}

#[cfg(feature = "embed")]
fn show_pad_evaluate_button(ui: &mut egui::Ui, app: &mut DebugApp) {
    let has_text = !app.state.utterance_text.trim().is_empty();
    let has_analyzer = app.pad_analyzer.is_some();
    let can_eval = has_text && has_analyzer;

    let btn = egui::Button::new("PAD 평가");
    let response = ui.add_enabled(can_eval, btn);

    let response = if !has_analyzer {
        response.on_disabled_hover_text(&app.embed_status)
    } else if !has_text {
        response.on_disabled_hover_text("대사를 입력하세요")
    } else {
        response
    };

    if response.clicked() {
        if let Some(ref mut analyzer) = app.pad_analyzer {
            use npc_mind::ports::UtteranceAnalyzer;
            match analyzer.analyze(app.state.utterance_text.trim()) {
                Ok(pad) => {
                    let prev_p = app.state.pad_pleasure;
                    let prev_a = app.state.pad_arousal;
                    let prev_d = app.state.pad_dominance;
                    app.state.pad_pleasure = pad.pleasure;
                    app.state.pad_arousal = pad.arousal;
                    app.state.pad_dominance = pad.dominance;
                    app.results.push(PipelineResult {
                        title: format!(
                            "[PAD 평가] \"{}\"",
                            app.state.utterance_text.chars().take(30).collect::<String>()
                        ),
                        sections: vec![pipeline::ResultSection {
                            heading: "임베딩 → PAD 변환 결과".into(),
                            content: format!(
                                "대사: \"{}\"\n\nP (Pleasure):  {prev_p:+.2} → {:+.3}\nA (Arousal):   {prev_a:+.2} → {:+.3}\nD (Dominance): {prev_d:+.2} → {:+.3}\n\n슬라이더에 자동 반영됨",
                                app.state.utterance_text,
                                pad.pleasure, pad.arousal, pad.dominance,
                            ),
                        }],
                        kind: pipeline::ResultKind::Incremental,
                    });
                }
                Err(e) => {
                    app.results.push(PipelineResult {
                        title: "[PAD 평가 오류]".into(),
                        sections: vec![pipeline::ResultSection {
                            heading: "오류".into(),
                            content: format!("{e}"),
                        }],
                        kind: pipeline::ResultKind::Incremental,
                    });
                }
            }
        }
    }
}

#[cfg(not(feature = "embed"))]
fn show_pad_evaluate_button(ui: &mut egui::Ui, _app: &mut DebugApp) {
    let btn = egui::Button::new("PAD 평가");
    ui.add_enabled(false, btn)
        .on_disabled_hover_text("embed feature 필요: --features devgui,embed");
}
