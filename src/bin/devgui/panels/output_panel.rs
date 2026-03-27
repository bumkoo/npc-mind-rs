//! 하단 결과 패널 — 가로 3열: 감정 상태 초기값 | 자극 적용 이력 | 지시/프롬프트
//!
//! 각 버튼이 특정 열만 제어하는 열별 상태 모델:
//! - 감정 평가 → Column 0 (중간 계산값, 감정 생성 추적, 초기 감정 상태)
//! - PAD 평가 / 자극 적용 → Column 1 (교대 누적, 최신 상단)
//! - 가이드 생성 / 대화 종료 → Column 2 (연기 지시, 프롬프트, 관계 갱신)

use eframe::egui;

use crate::app::DebugApp;
use crate::pipeline::Col1Entry;

const COL_TITLES: [&str; 3] = ["감정 상태 초기값", "자극 적용 이력", "지시/프롬프트"];

pub fn show(ui: &mut egui::Ui, app: &mut DebugApp) {
    // 헤더: 결과 로그 + 로그 지우기 + 상태 메시지
    ui.horizontal(|ui| {
        ui.heading("결과 로그");
        if ui.button("로그 지우기").clicked() {
            app.clear_all();
        }
        if !app.status_message.is_empty() {
            ui.label(
                egui::RichText::new(&app.status_message)
                    .weak()
                    .italics(),
            );
        }
    });
    ui.separator();

    // 결과가 없으면 안내 메시지
    let has_any = app.col0_appraisal.is_some()
        || !app.col1_entries.is_empty()
        || app.col2_guide.is_some()
        || app.col2_relationship.is_some();

    if !has_any {
        ui.centered_and_justified(|ui| {
            ui.label(egui::RichText::new("감정 평가를 실행하세요").weak());
        });
        return;
    }

    let total_width = ui.available_width();
    let col_widths = [total_width * 0.40, total_width * 0.27, total_width * 0.33];

    ui.horizontal_top(|ui| {
        // ── Column 0: 감정 상태 초기값 ──
        ui.allocate_ui_with_layout(
            egui::vec2(col_widths[0], ui.available_height()),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.set_max_width(col_widths[0]);
                ui.label(
                    egui::RichText::new(COL_TITLES[0])
                        .strong()
                        .size(13.0),
                );
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt("out_col_0")
                    .show(ui, |ui| {
                        if let Some(ref appraisal) = app.col0_appraisal {
                            render_section(ui, "중간 계산값 + 공식", &appraisal.intermediates);
                            render_section(ui, "감정 생성 추적", &appraisal.trace);
                            render_section(ui, "감정 상태", &appraisal.emotion_state);
                        } else {
                            ui.label(egui::RichText::new("—").weak());
                        }
                    });
            },
        );

        ui.separator();

        // ── Column 1: 자극 적용 이력 ──
        ui.allocate_ui_with_layout(
            egui::vec2(col_widths[1], ui.available_height()),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.set_max_width(col_widths[1]);
                ui.label(
                    egui::RichText::new(COL_TITLES[1])
                        .strong()
                        .size(13.0),
                );
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt("out_col_1")
                    .show(ui, |ui| {
                        if app.col1_entries.is_empty() {
                            ui.label(egui::RichText::new("—").weak());
                        } else {
                            // 역순 렌더링 (최신 상단)
                            for (i, entry) in app.col1_entries.iter().rev().enumerate() {
                                if i > 0 {
                                    ui.add_space(8.0);
                                    ui.separator();
                                }
                                match entry {
                                    Col1Entry::PadEval { content } => {
                                        render_section(ui, "임베딩 → PAD 변환 결과", content);
                                    }
                                    Col1Entry::Stimulus { delta, emotion_state } => {
                                        render_section(ui, "감정 상태", emotion_state);
                                        render_section(ui, "감정 변동", delta);
                                    }
                                }
                            }
                        }
                    });
            },
        );

        ui.separator();

        // ── Column 2: 지시/프롬프트 ──
        ui.allocate_ui_with_layout(
            egui::vec2(col_widths[2], ui.available_height()),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.set_max_width(col_widths[2]);
                ui.label(
                    egui::RichText::new(COL_TITLES[2])
                        .strong()
                        .size(13.0),
                );
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt("out_col_2")
                    .show(ui, |ui| {
                        let has_col2 = app.col2_guide.is_some()
                            || app.col2_relationship.is_some();

                        if let Some(ref guide) = app.col2_guide {
                            render_section(ui, "연기 지시", &guide.directive);
                            render_section(ui, "프롬프트", &guide.prompt);
                        }

                        if let Some(ref rel_text) = app.col2_relationship {
                            if app.col2_guide.is_some() {
                                ui.add_space(8.0);
                                ui.separator();
                            }
                            render_section(ui, "관계 갱신", rel_text);
                        }

                        if !has_col2 {
                            ui.label(egui::RichText::new("—").weak());
                        }
                    });
            },
        );
    });
}

/// 섹션 1개 렌더링: 제목(파란색) + 모노스페이스 텍스트 박스
fn render_section(ui: &mut egui::Ui, heading: &str, content: &str) {
    ui.label(
        egui::RichText::new(heading)
            .strong()
            .color(egui::Color32::from_rgb(120, 180, 255)),
    );
    let width = ui.available_width();
    egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
        ui.add(
            egui::TextEdit::multiline(&mut content.to_string())
                .font(egui::TextStyle::Monospace)
                .desired_width(width),
        );
    });
    ui.add_space(4.0);
}
