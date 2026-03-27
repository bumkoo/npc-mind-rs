//! DebugApp — eframe 앱 구현

use eframe::egui;

use npc_mind::domain::emotion::EmotionState;

use crate::panels;
use crate::pipeline::{self, AppraisalOutput, Col1Entry, GuideOutput};
use crate::state::GuiState;

pub struct DebugApp {
    pub state: GuiState,
    /// 현재 감정 상태 (자극 적용 시 체이닝용)
    pub current_emotion: Option<EmotionState>,

    // ── 열별 결과 상태 ──

    /// Column 0: 감정 상태 초기값
    pub col0_appraisal: Option<AppraisalOutput>,

    /// Column 1: 자극 적용 이력 (PAD 평가 / 자극 적용 교대 누적)
    pub col1_entries: Vec<Col1Entry>,
    /// PAD 평가 후 자극 미적용 상태
    pub has_unconsumed_pad: bool,

    /// Column 2: 가이드 출력
    pub col2_guide: Option<GuideOutput>,
    /// Column 2: 관계 갱신 텍스트 (하단)
    pub col2_relationship: Option<String>,

    /// 상태 표시줄 메시지
    pub status_message: String,

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
            current_emotion: None,
            col0_appraisal: None,
            col1_entries: Vec::new(),
            has_unconsumed_pad: false,
            col2_guide: None,
            col2_relationship: None,
            status_message: String::new(),
            #[cfg(feature = "embed")]
            pad_analyzer,
            #[cfg(feature = "embed")]
            embed_status,
        }
    }

    /// 모든 결과 상태 초기화
    pub fn clear_all(&mut self) {
        self.current_emotion = None;
        self.col0_appraisal = None;
        self.col1_entries.clear();
        self.has_unconsumed_pad = false;
        self.col2_guide = None;
        self.col2_relationship = None;
        self.status_message.clear();
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
            panels::output_panel::show(ui, self);
        });
    }
}

fn show_action_buttons(ui: &mut egui::Ui, app: &mut DebugApp) {
    ui.horizontal(|ui| {
        // ── 감정 평가 ──
        if ui.button("감정 평가").clicked() {
            app.clear_all();
            let (emotion, output) = pipeline::run_appraise(&app.state);
            app.current_emotion = Some(emotion);
            app.col0_appraisal = Some(output);
            app.status_message = app.col0_appraisal.as_ref().unwrap().title.clone();
        }

        ui.separator();

        // ── PAD 평가 (embed feature) ──
        show_pad_evaluate_button(ui, app);

        // ── 자극 적용 ──
        if ui.button("자극 적용").clicked() {
            if app.current_emotion.is_none() {
                app.status_message = "먼저 감정을 평가하세요".into();
            } else if !app.has_unconsumed_pad {
                app.status_message = "먼저 PAD 평가를 하세요".into();
            } else {
                let (new_emotion, delta, emotion_state) = pipeline::run_stimulus(
                    &app.state,
                    app.current_emotion.as_ref().unwrap(),
                );
                app.current_emotion = Some(new_emotion);
                app.col1_entries.push(Col1Entry::Stimulus { delta, emotion_state });
                app.has_unconsumed_pad = false;
                app.status_message = "[자극 적용] 완료".into();
            }
        }

        ui.separator();

        // ── 가이드 생성 ──
        if ui.button("가이드 생성").clicked() {
            if app.current_emotion.is_none() {
                app.status_message = "먼저 감정을 평가하세요".into();
            } else {
                let guide = pipeline::run_guide(
                    &app.state,
                    app.current_emotion.as_ref().unwrap(),
                );
                app.col2_guide = Some(guide);
                app.status_message = "[가이드 생성] 완료".into();
            }
        }

        ui.separator();

        // ── 대화 종료 ──
        if ui.button("대화 종료").clicked() {
            if app.current_emotion.is_none() {
                app.status_message = "먼저 감정을 평가하세요".into();
            } else {
                let (new_rel, text) = pipeline::run_after_dialogue(
                    &app.state,
                    app.current_emotion.as_ref().unwrap(),
                );
                app.state.closeness = new_rel.closeness().value();
                app.state.trust = new_rel.trust().value();
                app.state.power = new_rel.power().value();
                app.col2_relationship = Some(text);
                app.status_message = "[대화 종료] 관계 갱신 완료".into();
            }
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
        if app.current_emotion.is_none() {
            app.status_message = "먼저 감정을 평가하세요".into();
            return;
        }

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

                    let content = format!(
                        "대사: \"{}\"\n\nP (Pleasure):  {prev_p:+.2} → {:+.3}\nA (Arousal):   {prev_a:+.2} → {:+.3}\nD (Dominance): {prev_d:+.2} → {:+.3}\n\n슬라이더에 자동 반영됨",
                        app.state.utterance_text,
                        pad.pleasure, pad.arousal, pad.dominance,
                    );

                    if app.has_unconsumed_pad {
                        // 이전 PAD 미소비 → 마지막 PadEval 교체
                        if let Some(last) = app.col1_entries.last_mut() {
                            if matches!(last, Col1Entry::PadEval { .. }) {
                                *last = Col1Entry::PadEval { content };
                            } else {
                                app.col1_entries.push(Col1Entry::PadEval { content });
                            }
                        } else {
                            app.col1_entries.push(Col1Entry::PadEval { content });
                        }
                    } else {
                        app.col1_entries.push(Col1Entry::PadEval { content });
                    }
                    app.has_unconsumed_pad = true;
                    app.status_message = format!(
                        "[PAD 평가] \"{}\" P={:.1} A={:.1} D={:.1}",
                        app.state.utterance_text.chars().take(20).collect::<String>(),
                        pad.pleasure, pad.arousal, pad.dominance,
                    );
                }
                Err(e) => {
                    app.status_message = format!("[PAD 평가 오류] {e}");
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
