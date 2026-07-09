//! Governor — a deterministic PID servo (ported from `src/governor.ts`).
//! Authority rises on approve, falls on reject. Plant = authority dynamics.

#[derive(Clone, Copy, Debug)]
pub struct GovState {
    pub authority: f64,
    pub factor_status: &'static str,
    pub resonance_risky: bool,
    pub integral: f64,
    pub prev_error: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct GovConfig {
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,
    pub i_min: f64,
    pub i_max: f64,
    pub u_min: f64,
    pub u_max: f64,
    pub target_quality: f64,
    pub dead_ic: f64,
}

impl Default for GovConfig {
    fn default() -> Self {
        GovConfig {
            kp: 1.4,
            ki: 0.22,
            kd: 1.5,
            i_min: -1.0,
            i_max: 1.0,
            u_min: 0.0,
            u_max: 1.0,
            target_quality: 0.9,
            dead_ic: 0.02,
        }
    }
}

impl GovConfig {
    pub fn default_ck() -> Self {
        Self::default()
    }

    /// Step the servo. `quality` is the verified outcome (1 = approved, 0 = rejected).
    /// error = quality − target, so APPROVE (q=1) ⇒ error≈+0.1 ⇒ authority RISES;
    /// REJECT (q=0) ⇒ error≈−0.9 ⇒ authority FALLS. Authority is integrated toward
    /// 1.0 on a positive control signal, toward 0.0 on a negative one.
    pub fn step(&self, st: &mut GovState, quality: f64, _cost: f64, _volume: f64) {
        let error = quality - self.target_quality;
        st.integral += error;
        st.integral = st.integral.clamp(self.i_min, self.i_max);
        let derivative = error - st.prev_error;
        st.prev_error = error;
        let u = self.kp * error + self.ki * st.integral + self.kd * derivative;
        // authority moves with the signed control signal.
        st.authority = (st.authority + u).clamp(0.0, 1.0);
        st.factor_status = if u > 0.0 { "expand" } else { "contract" };
        st.resonance_risky = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authority_rises_on_approve() {
        // GREEN: approving (quality=1) raises authority.
        let cfg = GovConfig::default();
        let mut st = GovState {
            authority: 0.5,
            factor_status: "",
            resonance_risky: false,
            integral: 0.0,
            prev_error: 0.0,
        };
        cfg.step(&mut st, 1.0, 1e-18, 100.0);
        assert!(
            st.authority > 0.5,
            "authority did not rise on approve: {}",
            st.authority
        );
        assert_eq!(st.factor_status, "expand");
    }

    #[test]
    fn authority_falls_on_reject() {
        // RED: rejecting (quality=0) must lower authority.
        let cfg = GovConfig::default();
        let mut st = GovState {
            authority: 0.5,
            factor_status: "",
            resonance_risky: false,
            integral: 0.0,
            prev_error: 0.0,
        };
        cfg.step(&mut st, 0.0, 1e-18, 100.0);
        assert!(
            st.authority < 0.5,
            "authority did not fall on reject: {}",
            st.authority
        );
        assert_eq!(st.factor_status, "contract");
    }
}
