use naaf_core::context::RunContext;

use crate::transition::Transition;

pub fn run_transition(t: &Transition, ctx: &mut RunContext) -> Result<(), String> {
    let mut attempt = 0;

    loop {
        let output = t.executor.execute(ctx, vec![])?;

        let mut findings = vec![];

        for v in &t.validators {
            findings.extend(v.validate(&output)?);
        }

        if findings.is_empty() {
            return Ok(());
        }

        if attempt >= t.max_attempts {
            return Err("escalation".into());
        }

        // naive retry (replace later with remediation)
        attempt += 1;
    }
}
