//! NPC Mind 디버그 GUI — eframe 진입점

mod app;
mod panels;
mod pipeline;
mod presets;
mod state;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1500.0, 1200.0])
            .with_title("NPC Mind 디버그 GUI"),
        ..Default::default()
    };

    eframe::run_native(
        "NPC Mind 디버그 GUI",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Ok(Box::new(app::DebugApp::new(cc)))
        }),
    )
}

/// 한국어 폰트 설정 — 시스템 폰트(맑은 고딕) 로드
fn setup_fonts(ctx: &eframe::egui::Context) {
    let mut fonts = eframe::egui::FontDefinitions::default();

    // Windows 시스템 폰트 경로
    let font_paths = [
        "C:/Windows/Fonts/malgun.ttf",   // 맑은 고딕
        "C:/Windows/Fonts/gulim.ttc",    // 굴림 (폴백)
    ];

    for path in &font_paths {
        if let Ok(font_data) = std::fs::read(path) {
            fonts.font_data.insert(
                "korean".to_owned(),
                eframe::egui::FontData::from_owned(font_data).into(),
            );

            // Proportional, Monospace 모두에 한국어 폰트 추가
            if let Some(family) = fonts.families.get_mut(&eframe::egui::FontFamily::Proportional) {
                family.insert(0, "korean".to_owned());
            }
            if let Some(family) = fonts.families.get_mut(&eframe::egui::FontFamily::Monospace) {
                family.push("korean".to_owned());
            }

            ctx.set_fonts(fonts);
            return;
        }
    }

    // 폰트를 찾지 못하면 기본 폰트로 진행 (CJK 깨질 수 있음)
    eprintln!("[경고] 한국어 폰트를 찾지 못했습니다. CJK 문자가 깨질 수 있습니다.");
}
