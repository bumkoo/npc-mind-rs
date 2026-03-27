//! 상단 입력 패널 — 가로 2열 배치, 폰트 +1

use eframe::egui;

use crate::state::*;
use crate::presets;

/// 상단 입력 패널을 렌더링합니다.
pub fn show(ui: &mut egui::Ui, state: &mut GuiState) {
    // 입력 패널 폰트 +1
    let base = ui.style().text_styles[&egui::TextStyle::Body].size;
    ui.style_mut().text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(base + 1.0));
    ui.style_mut().text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(base + 1.0));

    // 프리셋 (전폭)
    ui.horizontal(|ui| {
        ui.label("프리셋:");
        let prev = state.selected_preset;
        egui::ComboBox::from_id_salt("preset")
            .selected_text(state.selected_preset.label())
            .show_ui(ui, |ui| {
                for &p in PresetChoice::ALL {
                    ui.selectable_value(&mut state.selected_preset, p, p.label());
                }
            });
        if state.selected_preset != prev && state.selected_preset != PresetChoice::Custom {
            presets::apply_preset(state, state.selected_preset);
        }
    });
    ui.separator();

    // 가로 2열: 왼쪽 30% (NPC+HEXACO) | 오른쪽 70% (상황+관계+PAD)
    let total_width = ui.available_width();
    let left_width = total_width * 0.50;

    ui.horizontal_top(|ui| {
        // ── 왼쪽: NPC 정보 + HEXACO ──
        ui.allocate_ui_with_layout(
            egui::vec2(left_width, ui.available_height()),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.set_max_width(left_width);
                egui::ScrollArea::vertical()
                    .id_salt("input_left")
                    .show(ui, |ui| {
                        ui.set_max_width(left_width);
                        show_npc_info(ui, state);
                        ui.separator();
                        show_hexaco(ui, state);
                    });
            },
        );

        ui.separator();

        // ── 오른쪽: 상황 + 관계 + PAD ──
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), ui.available_height()),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("input_right")
                    .show(ui, |ui| {
                        show_situation(ui, state);
                        ui.separator();
                        show_relationship(ui, state);
                        ui.separator();
                        show_pad(ui, state);
                    });
            },
        );
    });
}

// ---------------------------------------------------------------------------
// NPC 정보
// ---------------------------------------------------------------------------

fn show_npc_info(ui: &mut egui::Ui, state: &mut GuiState) {
    egui::CollapsingHeader::new("NPC 정보")
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("ID:");
                ui.add(egui::TextEdit::singleline(&mut state.npc_id).desired_width(100.0));
                ui.label("이름:");
                ui.add(egui::TextEdit::singleline(&mut state.npc_name).desired_width(120.0));
            });
            ui.horizontal(|ui| {
                ui.label("설명:");
                ui.add(egui::TextEdit::singleline(&mut state.npc_description).desired_width(ui.available_width()));
            });
        });
}

// ---------------------------------------------------------------------------
// HEXACO 슬라이더
// ---------------------------------------------------------------------------

/// 1단: 타이틀+평균 | 2단: 4 facet 슬라이더 1줄 (컴팩트)
macro_rules! dimension_row {
    ($ui:expr, $title:expr, [($l1:expr, $v1:expr), ($l2:expr, $v2:expr), ($l3:expr, $v3:expr), ($l4:expr, $v4:expr) $(,)?]) => {{
        let avg = ($v1 + $v2 + $v3 + $v4) / 4.0;
        $ui.horizontal(|ui| {
            ui.strong($title);
            ui.label(format!("{avg:.2}"));
        });
        $ui.horizontal(|ui| {
            ui.spacing_mut().slider_width = 85.0;
            ui.spacing_mut().item_spacing.x = 14.0;
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.label($l1);
            ui.add(egui::Slider::new(&mut $v1, -1.0..=1.0).step_by(0.05).fixed_decimals(2));
            ui.label($l2);
            ui.add(egui::Slider::new(&mut $v2, -1.0..=1.0).step_by(0.05).fixed_decimals(2));
            ui.label($l3);
            ui.add(egui::Slider::new(&mut $v3, -1.0..=1.0).step_by(0.05).fixed_decimals(2));
            ui.label($l4);
            ui.add(egui::Slider::new(&mut $v4, -1.0..=1.0).step_by(0.05).fixed_decimals(2));
        });
        $ui.add_space(1.0);
    }};
}

fn show_hexaco(ui: &mut egui::Ui, state: &mut GuiState) {
    dimension_row!(ui, "H: Honesty-Humility 정직-겸손성",
        [("진실성", state.sincerity), ("공정성", state.fairness),
         ("탐욕회피", state.greed_avoidance), ("겸손", state.modesty)]);
    dimension_row!(ui, "E: Emotionality 정서성",
        [("공포성", state.fearfulness), ("불안", state.anxiety),
         ("의존성", state.dependence), ("감상성", state.sentimentality)]);
    dimension_row!(ui, "X: Extraversion 외향성",
        [("자존감", state.social_self_esteem), ("대담성", state.social_boldness),
         ("사교성", state.sociability), ("활력", state.liveliness)]);
    dimension_row!(ui, "A: Agreeableness 원만성",
        [("용서", state.forgiveness), ("온화함", state.gentleness),
         ("유연성", state.flexibility), ("인내심", state.patience)]);
    dimension_row!(ui, "C: Conscientiousness 성실성",
        [("조직성", state.organization), ("근면성", state.diligence),
         ("완벽주의", state.perfectionism), ("신중함", state.prudence)]);
    dimension_row!(ui, "O: Openness 경험개방성",
        [("미적감상", state.aesthetic_appreciation), ("탐구심", state.inquisitiveness),
         ("창의성", state.creativity), ("비관습성", state.unconventionality)]);
}

// ---------------------------------------------------------------------------
// 상황 설정
// ---------------------------------------------------------------------------

fn show_situation(ui: &mut egui::Ui, state: &mut GuiState) {
    ui.horizontal(|ui| {
        ui.strong("상황 설정");
        ui.label(" — 설명:");
    });
    ui.add(
        egui::TextEdit::multiline(&mut state.situation_description)
            .desired_rows(2)
            .desired_width(ui.available_width() * 0.8),
    );

    let mut remove_idx = None;
    let num_focuses = state.focuses.len();
    for i in 0..num_focuses {
        ui.horizontal(|ui| {
            ui.label(format!("F{}:", i + 1));
            egui::ComboBox::from_id_salt(format!("focus_type_{i}"))
                .width(80.0)
                .selected_text(state.focuses[i].focus_type.label())
                .show_ui(ui, |ui| {
                    for &ft in FocusType::ALL {
                        ui.selectable_value(&mut state.focuses[i].focus_type, ft, ft.label());
                    }
                });

            let focus = &mut state.focuses[i];
            match focus.focus_type {
                FocusType::Event => {
                    ui.label("자기:");
                    ui.add(egui::Slider::new(&mut focus.desirability_for_self, -1.0..=1.0).step_by(0.05).fixed_decimals(2));
                    ui.checkbox(&mut focus.has_other, "타인");
                    ui.label("전망:");
                    egui::ComboBox::from_id_salt(format!("prospect_{i}"))
                        .width(100.0)
                        .selected_text(focus.prospect.label())
                        .show_ui(ui, |ui| {
                            for &p in ProspectChoice::ALL {
                                ui.selectable_value(&mut focus.prospect, p, p.label());
                            }
                        });
                }
                FocusType::Action => {
                    ui.checkbox(&mut focus.is_self_agent, "자기");
                    ui.label("도덕성:");
                    ui.add(egui::Slider::new(&mut focus.praiseworthiness, -1.0..=1.0).step_by(0.05).fixed_decimals(2));
                }
                FocusType::Object => {
                    ui.label("매력도:");
                    ui.add(egui::Slider::new(&mut focus.appealingness, -1.0..=1.0).step_by(0.05).fixed_decimals(2));
                }
            }

            if num_focuses > 1 && ui.small_button("X").clicked() {
                remove_idx = Some(i);
            }
        });

        // Event 타인 영향 서브라인 (has_other 체크 시)
        if state.focuses[i].focus_type == FocusType::Event && state.focuses[i].has_other {
            let focus = &mut state.focuses[i];
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label("대상:");
                ui.add(egui::TextEdit::singleline(&mut focus.other_target_id).desired_width(60.0));
                ui.label("영향:");
                ui.add(egui::Slider::new(&mut focus.desirability_for_other, -1.0..=1.0).step_by(0.05).fixed_decimals(2));
                ui.label("친밀:");
                ui.add(egui::Slider::new(&mut focus.other_closeness, -1.0..=1.0).step_by(0.05).fixed_decimals(2));
                ui.label("신뢰:");
                ui.add(egui::Slider::new(&mut focus.other_trust, -1.0..=1.0).step_by(0.05).fixed_decimals(2));
            });
        }
    }

    if let Some(idx) = remove_idx {
        state.focuses.remove(idx);
    }

    ui.horizontal(|ui| {
        if ui.small_button("+ 포커스").clicked() {
            state.focuses.push(FocusEntry::default());
        }
    });
}

// ---------------------------------------------------------------------------
// 대화 상대 관계 (2줄)
// ---------------------------------------------------------------------------

fn show_relationship(ui: &mut egui::Ui, state: &mut GuiState) {
    ui.horizontal(|ui| {
        ui.strong("대화 상대");
        ui.label("소유자:");
        ui.add(egui::TextEdit::singleline(&mut state.rel_owner_id).desired_width(70.0));
        ui.label("상대방:");
        ui.add(egui::TextEdit::singleline(&mut state.rel_target_id).desired_width(70.0));
    });
    ui.horizontal(|ui| {
        ui.label("친밀도:");
        ui.add(egui::Slider::new(&mut state.closeness, -1.0..=1.0).step_by(0.05).fixed_decimals(2));
        ui.label("신뢰도:");
        ui.add(egui::Slider::new(&mut state.trust, -1.0..=1.0).step_by(0.05).fixed_decimals(2));
        ui.label("상하:");
        ui.add(egui::Slider::new(&mut state.power, -1.0..=1.0).step_by(0.05).fixed_decimals(2));
    });
}

// ---------------------------------------------------------------------------
// PAD 자극 (대사 + PAD 한 줄)
// ---------------------------------------------------------------------------

fn show_pad(ui: &mut egui::Ui, state: &mut GuiState) {
    ui.horizontal(|ui| {
        ui.strong("PAD 자극");
        ui.label(" — 대사:");
    });
    ui.add(
        egui::TextEdit::multiline(&mut state.utterance_text)
            .desired_rows(2)
            .desired_width(ui.available_width() * 0.8),
    );
    ui.horizontal(|ui| {
        ui.label("P:");
        ui.add(egui::Slider::new(&mut state.pad_pleasure, -1.0..=1.0).step_by(0.05).fixed_decimals(2));
        ui.label("A:");
        ui.add(egui::Slider::new(&mut state.pad_arousal, -1.0..=1.0).step_by(0.05).fixed_decimals(2));
        ui.label("D:");
        ui.add(egui::Slider::new(&mut state.pad_dominance, -1.0..=1.0).step_by(0.05).fixed_decimals(2));
    });
}

