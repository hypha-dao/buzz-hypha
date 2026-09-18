//! Who may send which Shapers command (Protocol §3.2, §6.4).
//!
//! Pure decisions over the live `39103` content: no database, no clock
//! other than the `now` the caller passes. A refusal carries the exact wire
//! text §3.2 fixes — `restricted: <reason>` when the author lacks the role,
//! `invalid: <reason>` when the target is in the wrong state — so the
//! handlers in [`super::shapers`] only route and write.

use buzz_core::intelligent_org::{OfferedSeat, Shapers, ShapersOp};

use crate::handlers::ingest::IngestError;

fn restricted(reason: &str) -> IngestError {
    IngestError::Rejected(format!("restricted: {reason}"))
}

fn invalid(reason: &str) -> IngestError {
    IngestError::Rejected(format!("invalid: {reason}"))
}

/// Whether `actor` holds a live seat.
pub fn is_shaper(shapers: &Shapers, actor: &str) -> bool {
    shapers.shapers.iter().any(|p| p == actor)
}

/// A command only a Shaper may send.
pub fn require_shaper(shapers: &Shapers, actor: &str) -> Result<(), IngestError> {
    if is_shaper(shapers, actor) {
        Ok(())
    } else {
        Err(restricted("not a Shaper"))
    }
}

/// The one `io_shapers_propose` accepted while no `39103` exists (§6.4
/// Bootstrap): `op=add`, naming the sender, from the community owner.
pub fn bootstrap(
    op: ShapersOp,
    subject: Option<&str>,
    actor: &str,
    actor_is_owner: bool,
) -> Result<(), IngestError> {
    if op != ShapersOp::Add {
        return Err(invalid(
            "no Shapers yet; the community owner bootstraps with op=add naming themselves",
        ));
    }
    if subject != Some(actor) {
        return Err(invalid("bootstrap must name the owner themselves"));
    }
    if !actor_is_owner {
        return Err(restricted("only the community owner may bootstrap"));
    }
    Ok(())
}

/// `io_shaper_step_down`: a Shaper, unless they are the last one.
pub fn step_down(shapers: &Shapers, actor: &str) -> Result<(), IngestError> {
    require_shaper(shapers, actor)?;
    if shapers.shapers.len() == 1 {
        return Err(restricted("the last Shaper cannot step down"));
    }
    Ok(())
}

/// `io_shaper_accept`: the `p` of a passed `shapers/add` whose seat is still
/// offered. Returns the seat being taken.
pub fn accept_seat<'a>(
    shapers: &'a Shapers,
    actor: &str,
    proposal: &str,
    now: u64,
) -> Result<&'a OfferedSeat, IngestError> {
    if is_shaper(shapers, actor) {
        return Err(invalid("already a Shaper"));
    }
    let seat = shapers
        .offered
        .iter()
        .find(|seat| seat.p == actor && seat.proposal == proposal)
        .ok_or_else(|| restricted("no seat is offered to you on that proposal"))?;
    if now >= seat.at.saturating_add(shapers.offer_window_secs) {
        return Err(invalid("the offer has lapsed"));
    }
    Ok(seat)
}

#[cfg(test)]
mod tests {
    use super::*;
    use buzz_core::intelligent_org::{
        DecisionRules, DEFAULT_DECISION_WINDOW_SECS, DEFAULT_OFFER_WINDOW_SECS,
    };

    fn pk(seed: u8) -> String {
        hex::encode([seed; 32])
    }

    fn shapers(seats: &[u8], offered: &[(u8, &str, u64)]) -> Shapers {
        Shapers {
            founder: pk(1),
            shapers: seats.iter().map(|s| pk(*s)).collect(),
            offered: offered
                .iter()
                .map(|(p, proposal, at)| OfferedSeat {
                    p: pk(*p),
                    proposal: (*proposal).to_owned(),
                    at: *at,
                })
                .collect(),
            room: None,
            agent: None,
            agent_hosted: false,
            rules: DecisionRules::default(),
            decision_window_secs: DEFAULT_DECISION_WINDOW_SECS,
            offer_window_secs: DEFAULT_OFFER_WINDOW_SECS,
            updated_at: 0,
            receipt: pk(9),
        }
    }

    fn message(result: Result<impl std::fmt::Debug, IngestError>) -> String {
        match result {
            Err(IngestError::Rejected(m)) => m,
            other => panic!("expected a rejection, got {other:?}"),
        }
    }

    #[test]
    fn bootstrap_needs_add_naming_the_owner_from_the_owner() {
        assert!(bootstrap(ShapersOp::Add, Some(&pk(1)), &pk(1), true).is_ok());
        assert_eq!(
            message(bootstrap(ShapersOp::Remove, Some(&pk(1)), &pk(1), true)),
            "invalid: no Shapers yet; the community owner bootstraps with op=add naming themselves"
        );
        assert_eq!(
            message(bootstrap(ShapersOp::Add, Some(&pk(2)), &pk(1), true)),
            "invalid: bootstrap must name the owner themselves"
        );
        assert_eq!(
            message(bootstrap(ShapersOp::Add, None, &pk(1), true)),
            "invalid: bootstrap must name the owner themselves"
        );
        assert_eq!(
            message(bootstrap(ShapersOp::Add, Some(&pk(1)), &pk(1), false)),
            "restricted: only the community owner may bootstrap"
        );
    }

    #[test]
    fn step_down_needs_a_seat_and_another_shaper() {
        let two = shapers(&[1, 2], &[]);
        assert!(step_down(&two, &pk(1)).is_ok());
        assert_eq!(message(step_down(&two, &pk(3))), "restricted: not a Shaper");
        let one = shapers(&[1], &[]);
        assert_eq!(
            message(step_down(&one, &pk(1))),
            "restricted: the last Shaper cannot step down"
        );
    }

    #[test]
    fn accept_takes_only_a_live_seat_offered_to_the_sender() {
        let s = shapers(&[1], &[(2, "prop-a", 1_000)]);
        let seat = accept_seat(&s, &pk(2), "prop-a", 1_500).expect("seat");
        assert_eq!(seat.proposal, "prop-a");
        assert_eq!(
            message(accept_seat(&s, &pk(2), "prop-b", 1_500)),
            "restricted: no seat is offered to you on that proposal"
        );
        assert_eq!(
            message(accept_seat(&s, &pk(3), "prop-a", 1_500)),
            "restricted: no seat is offered to you on that proposal"
        );
        assert_eq!(
            message(accept_seat(&s, &pk(1), "prop-a", 1_500)),
            "invalid: already a Shaper"
        );
        assert_eq!(
            message(accept_seat(
                &s,
                &pk(2),
                "prop-a",
                1_000 + DEFAULT_OFFER_WINDOW_SECS
            )),
            "invalid: the offer has lapsed"
        );
        assert!(accept_seat(&s, &pk(2), "prop-a", 999 + DEFAULT_OFFER_WINDOW_SECS).is_ok());
    }
}
