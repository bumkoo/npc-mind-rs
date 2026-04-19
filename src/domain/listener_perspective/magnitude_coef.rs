//! Magnitude 계수 테이블 + bin 경계
//!
//! Listener-perspective 변환식:
//! ```text
//! P_L = sign × coef_p[magnitude] × P_S
//! A_L = coef_a[magnitude] × A_S   (부호 유지)
//! D_L = coef_d[magnitude] × D_S   (부호 유지)
//! ```
//!
//! 설계: `docs/emotion/sign-classifier-design.md` §3.1, §3.1.2
//!       `docs/emotion/phase7-converter-integration.md` §3.1

use super::types::Magnitude;

/// Magnitude 별 축별 계수 테이블
///
/// 기본값 (sign-classifier-design.md §3.1 Phase 2 baseline):
///
/// | magnitude | P   | A   | D   | 대응 화행                |
/// |-----------|-----|-----|-----|--------------------------|
/// | weak      | 0.5 | 0.5 | 0.4 | 사과·간청·위로 (감쇄)    |
/// | normal    | 1.0 | 1.0 | 1.0 | 감사·칭찬·중립 (기준)    |
/// | strong    | 1.5 | 1.3 | 1.3 | 비난·위협·빈정 (증폭)    |
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MagnitudeCoefTable {
    pub weak_p: f32,
    pub normal_p: f32,
    pub strong_p: f32,
    pub weak_a: f32,
    pub normal_a: f32,
    pub strong_a: f32,
    pub weak_d: f32,
    pub normal_d: f32,
    pub strong_d: f32,
}

impl Default for MagnitudeCoefTable {
    /// Phase 2 baseline 계수 (sign-classifier-design.md §3.1)
    fn default() -> Self {
        Self {
            weak_p: 0.5,
            normal_p: 1.0,
            strong_p: 1.5,
            weak_a: 0.5,
            normal_a: 1.0,
            strong_a: 1.3,
            weak_d: 0.4,
            normal_d: 1.0,
            strong_d: 1.3,
        }
    }
}

impl MagnitudeCoefTable {
    /// 주어진 magnitude 에 해당하는 P축 계수
    pub fn p_coef(&self, magnitude: Magnitude) -> f32 {
        match magnitude {
            Magnitude::Weak => self.weak_p,
            Magnitude::Normal => self.normal_p,
            Magnitude::Strong => self.strong_p,
        }
    }

    /// 주어진 magnitude 에 해당하는 A축 계수
    pub fn a_coef(&self, magnitude: Magnitude) -> f32 {
        match magnitude {
            Magnitude::Weak => self.weak_a,
            Magnitude::Normal => self.normal_a,
            Magnitude::Strong => self.strong_a,
        }
    }

    /// 주어진 magnitude 에 해당하는 D축 계수
    pub fn d_coef(&self, magnitude: Magnitude) -> f32 {
        match magnitude {
            Magnitude::Weak => self.weak_d,
            Magnitude::Normal => self.normal_d,
            Magnitude::Strong => self.strong_d,
        }
    }
}

/// Magnitude bin 경계 (산출된 |P_L| 값을 라벨과 비교할 때 사용)
///
/// 기본값:
/// - weak: |P_L| < 0.15
/// - normal: 0.15 ≤ |P_L| < 0.4
/// - strong: |P_L| ≥ 0.4
///
/// 이 구조체는 주로 **검증·테스트·리포트** 용도.
/// Converter 는 분류기가 정한 magnitude 를 직접 사용 (bin 분류 불필요).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MagnitudeBinThresholds {
    pub weak_max: f32,
    pub normal_max: f32,
}

impl Default for MagnitudeBinThresholds {
    fn default() -> Self {
        Self {
            weak_max: 0.15,
            normal_max: 0.4,
        }
    }
}

impl MagnitudeBinThresholds {
    /// |P_L| 절대값을 magnitude bin 에 매핑
    pub fn bin_of(&self, abs_p_l: f32) -> Magnitude {
        let a = abs_p_l.abs();
        if a < self.weak_max {
            Magnitude::Weak
        } else if a < self.normal_max {
            Magnitude::Normal
        } else {
            Magnitude::Strong
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_coefs_match_design_doc() {
        let t = MagnitudeCoefTable::default();
        assert_eq!(t.p_coef(Magnitude::Weak), 0.5);
        assert_eq!(t.p_coef(Magnitude::Normal), 1.0);
        assert_eq!(t.p_coef(Magnitude::Strong), 1.5);
        assert_eq!(t.a_coef(Magnitude::Weak), 0.5);
        assert_eq!(t.a_coef(Magnitude::Strong), 1.3);
        assert_eq!(t.d_coef(Magnitude::Weak), 0.4);
        assert_eq!(t.d_coef(Magnitude::Strong), 1.3);
    }

    #[test]
    fn custom_coef_table_overrides() {
        let custom = MagnitudeCoefTable {
            weak_p: 0.3,
            normal_p: 1.0,
            strong_p: 2.0,
            ..Default::default()
        };
        assert_eq!(custom.p_coef(Magnitude::Weak), 0.3);
        assert_eq!(custom.p_coef(Magnitude::Strong), 2.0);
        // 나머지는 default
        assert_eq!(custom.a_coef(Magnitude::Strong), 1.3);
    }

    #[test]
    fn bin_boundaries() {
        let b = MagnitudeBinThresholds::default();
        assert_eq!(b.bin_of(0.0), Magnitude::Weak);
        assert_eq!(b.bin_of(0.14), Magnitude::Weak);
        assert_eq!(b.bin_of(0.15), Magnitude::Normal);
        assert_eq!(b.bin_of(0.39), Magnitude::Normal);
        assert_eq!(b.bin_of(0.4), Magnitude::Strong);
        assert_eq!(b.bin_of(0.8), Magnitude::Strong);
        assert_eq!(b.bin_of(-0.5), Magnitude::Strong); // 절대값 기반
    }
}
