pub const PLAN: &str = include_str!("../templates/plan.md");
pub const DEFAULT: &str = include_str!("../templates/default.md");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mode_does_not_turn_discussion_into_implementation() {
        assert!(DEFAULT.contains(
            "does not turn a question, audit or plan-only request into authorization to edit"
        ));
        assert!(DEFAULT.contains("Silence never grants permission"));
        assert!(DEFAULT.contains("Stop when the requested outcome is achieved"));
    }

    #[test]
    fn plan_mode_preserves_non_mutation_and_material_clarification() {
        assert!(PLAN.contains("Do not edit source or configuration"));
        assert!(PLAN.contains("Explore discoverable facts before asking the user"));
        assert!(PLAN.contains("Ask one focused question"));
        assert!(PLAN.contains("Subagents require the user's explicit permission"));
        assert!(PLAN.contains("<proposed_plan>"));
    }
}
