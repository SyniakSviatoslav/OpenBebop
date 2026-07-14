//! Router — the token/model router (ported from the AGENTS.md TOKEN ROUTER rule).
//!
//! Classify a request → route to the CHEAPEST ADEQUATE backend. Deterministic:
//! no RNG, no Date, no network. The map is the operator's standing directive
//! ("classify → cheapest adequate route; narrow grants").

/// Request classification for the router.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Explore, // read-only nav / file read
    Doer,    // cheap deterministic action
    Reason,  // reasoning / synthesis
    Review,  // verification / audit
}

impl Class {
    /// Classify free text into a route class. Deterministic keyword map.
    pub fn classify(task: &str) -> Class {
        let t = task.to_ascii_lowercase();
        if t.contains("audit")
            || t.contains("verify")
            || t.contains("review")
            || t.contains("security")
        {
            Class::Review
        } else if t.contains("design")
            || t.contains("plan")
            || t.contains("reason")
            || t.contains("why")
            || t.contains("decide")
            || t.contains("synthesize")
        {
            Class::Reason
        } else if t.contains("read")
            || t.contains("find")
            || t.contains("search")
            || t.contains("look")
            || t.contains("show")
            || t.contains("list")
        {
            Class::Explore
        } else {
            Class::Doer
        }
    }
}

/// The cheapest adequate backend for a class.
pub fn route(task: &str) -> &'static str {
    match Class::classify(task) {
        Class::Explore => "haiku", // cheap nav
        Class::Doer => "haiku",    // cheap deterministic action
        Class::Reason => "sonnet", // mid reasoning
        Class::Review => "opus",   // heaviest verification only when needed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explore_routes_cheap() {
        // GREEN: a read-only task stays on the cheapest backend.
        assert_eq!(route("read the config file"), "haiku");
        assert_eq!(route("find the module graph"), "haiku");
    }

    #[test]
    fn review_routes_heavy() {
        // GREEN: verification work gets the strongest backend (narrow grant).
        assert_eq!(route("security audit the auth flow"), "opus");
        assert_eq!(route("review the pr"), "opus");
    }

    #[test]
    fn reason_routes_mid() {
        assert_eq!(route("design the planner"), "sonnet");
        assert_eq!(route("reason about the failure"), "sonnet");
    }

    #[test]
    fn default_is_doer_not_reason() {
        // RED+GREEN: an ambiguous task must NOT balloon to the heaviest model.
        // Cheapest adequate = haiku (doer), not opus.
        assert_eq!(route("do the thing"), "haiku");
        assert_ne!(route("do the thing"), "opus");
    }

    #[test]
    fn classify_is_deterministic() {
        let a = Class::classify("review the auth");
        let b = Class::classify("REVIEW the AUTH");
        assert_eq!(a, b);
    }
}
