//! 하단 결과 패널 — 가로 3열: 계산값 | 감정추적+상태 | 지시+프롬프트
//!
//! Base 결과(감정 평가/가이드 생성)를 기준으로 3열을 채우고,
//! 이후 Incremental 결과(PAD 평가/자극 적용/대화 종료)는
//! 해당 열 하단에 누적 표시한다.

use eframe::egui;

use crate::pipeline::{PipelineResult, ResultKind};

/// 섹션을 3열로 분류하는 기준
fn classify_section(heading: &str) -> usize {
    match heading {
        "중간 계산값 + 공식" => 0,
        "감정 생성 추적" | "감정 상태" | "감정 변동" => 1,
        "연기 지시" | "프롬프트" => 2,
        "PAD 자극" | "상대방 대사" => 1,
        "관계 갱신" => 2,
        "임베딩 → PAD 변환 결과" => 1,
        _ => 1,
    }
}

const COL_TITLES: [&str; 3] = ["계산값", "감정 추적/상태", "지시/프롬프트"];

/// results에서 마지막 Base 인덱스를 찾는다
fn find_last_base(results: &[PipelineResult]) -> Option<usize> {
    results
        .iter()
        .rposition(|r| r.kind == ResultKind::Base)
}

pub fn show(ui: &mut egui::Ui, results: &mut Vec<PipelineResult>) {
    ui.horizontal(|ui| {
        ui.heading("결과 로그");
        if ui.button("로그 지우기").clicked() {
            results.clear();
        }
        if let Some(last) = results.last() {
            ui.label(
                egui::RichText::new(&last.title)
                    .weak()
                    .italics(),
            );
        }
    });
    ui.separator();

    if results.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(egui::RichText::new("감정 평가 또는 가이드 생성을 실행하세요").weak());
        });
        return;
    }

    // Base가 없으면(Incremental만 있으면) 마지막 결과를 단독 표시
    let base_idx = find_last_base(results);
    let (base_result_idx, incremental_range) = match base_idx {
        Some(idx) => (idx, (idx + 1)..results.len()),
        None => (results.len() - 1, results.len()..results.len()),
    };

    // Base 결과에서 3열 분류
    let base = &results[base_result_idx];
    let mut cols: [Vec<(&str, &str)>; 3] = [vec![], vec![], vec![]];
    for section in &base.sections {
        let col = classify_section(&section.heading);
        cols[col].push((&section.heading, &section.content));
    }

    // Incremental 결과들의 섹션도 해당 열에 추가 (타이틀 구분선 포함)
    // 임시로 제목+내용 문자열을 모아둔다
    let mut incremental_cols: [Vec<(String, Vec<(&str, &str)>)>; 3] =
        [vec![], vec![], vec![]];
    for i in incremental_range {
        let result = &results[i];
        let mut per_col: [Vec<(&str, &str)>; 3] = [vec![], vec![], vec![]];
        for section in &result.sections {
            let col = classify_section(&section.heading);
            per_col[col].push((&section.heading, &section.content));
        }
        for col_idx in 0..3 {
            if !per_col[col_idx].is_empty() {
                incremental_cols[col_idx].push((
                    result.title.clone(),
                    std::mem::take(&mut per_col[col_idx]),
                ));
            }
        }
    }

    let total_width = ui.available_width();
    let col_widths = [total_width * 0.30, total_width * 0.35, total_width * 0.35];

    ui.horizontal_top(|ui| {
        for (col_idx, base_sections) in cols.iter().enumerate() {
            ui.allocate_ui_with_layout(
                egui::vec2(col_widths[col_idx], ui.available_height()),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.set_max_width(col_widths[col_idx]);
                    ui.label(
                        egui::RichText::new(COL_TITLES[col_idx])
                            .strong()
                            .size(13.0),
                    );
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .id_salt(format!("out_col_{col_idx}"))
                        .show(ui, |ui| {
                            // Base 섹션 렌더링
                            render_sections(ui, base_sections);

                            // Incremental 섹션 누적 렌더링
                            for (title, sections) in &incremental_cols[col_idx] {
                                ui.add_space(8.0);
                                ui.separator();
                                ui.label(
                                    egui::RichText::new(title)
                                        .weak()
                                        .italics()
                                        .size(11.0),
                                );
                                ui.add_space(2.0);
                                render_sections(ui, sections);
                            }

                            if base_sections.is_empty()
                                && incremental_cols[col_idx].is_empty()
                            {
                                ui.label(egui::RichText::new("—").weak());
                            }
                        });
                },
            );
            if col_idx < 2 {
                ui.separator();
            }
        }
    });

    // 이전 결과 히스토리 (접힘) — Base 이전 결과만 표시
    let history_count = if let Some(idx) = base_idx {
        idx
    } else {
        results.len().saturating_sub(1)
    };
    if history_count > 0 {
        ui.separator();
        egui::CollapsingHeader::new(format!("이전 결과 ({}건)", history_count))
            .default_open(false)
            .show(ui, |ui| {
                for (i, result) in results.iter().take(history_count).rev().enumerate() {
                    ui.label(
                        egui::RichText::new(&result.title)
                            .weak()
                            .italics(),
                    );
                    if i >= 9 {
                        ui.label("...");
                        break;
                    }
                }
            });
    }
}

fn render_sections(ui: &mut egui::Ui, sections: &[(&str, &str)]) {
    for (heading, content) in sections {
        ui.label(
            egui::RichText::new(*heading)
                .strong()
                .color(egui::Color32::from_rgb(120, 180, 255)),
        );
        egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut (*content).to_string())
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY),
            );
        });
        ui.add_space(4.0);
    }
}
