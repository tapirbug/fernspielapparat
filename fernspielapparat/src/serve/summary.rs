use crate::evt::Event as MachineEventWithState;
use crate::senses::Input;
use crate::states::{State, Symbol};

use serde::Serialize;

type MachineEvent<'a> = MachineEventWithState<'a, State>;

/// Used by the server to describe an event through the
/// `fernspielevt` protocol.
#[derive(Serialize, Clone, PartialEq, Debug)]
#[serde(tag = "type")]
pub enum FernspielEvent {
    /// Either the phonebook just started with this initial
    /// state or it reached the initial state again, resetting
    /// the phonebook.
    ///
    /// If this is not the first start, the event is preceded by
    /// a normal transition event.
    #[serde(rename = "start")]
    Start { initial: StateSummary },
    /// A terminal state has been reached by the user progressing
    /// through states.
    #[serde(rename = "finish")]
    Finish { terminal: StateSummary },
    #[serde(rename = "transition")]
    Transition {
        /// The trigger for this transition.
        reason: TransitionCause,
        /// `None` on reset or newly started.
        from: StateSummary,
        /// The new current state.
        to: StateSummary,
    },
}

#[derive(Serialize, Clone, PartialEq, Debug)]
pub enum TransitionCause {
    /// Transition in response to actuator idleness for the
    /// contained amount of time.
    #[serde(rename = "timeout")]
    Timeout(f64),
    /// User has typed or dialed through the phone dial or
    /// keyboard.
    #[serde(rename = "dial")]
    Dial(String),
}

impl<'a> From<&MachineEvent<'a>> for FernspielEvent {
    fn from(event: &MachineEvent) -> Self {
        match event {
            MachineEvent::Start { initial } => FernspielEvent::Start {
                initial: (*initial).into(),
            },
            MachineEvent::Finish { terminal } => FernspielEvent::Finish {
                terminal: (*terminal).into(),
            },
            MachineEvent::Transition { cause, from, to } => FernspielEvent::Transition {
                reason: match cause {
                    Symbol::Dial(input) => TransitionCause::Dial(match input {
                        Input::Digit(num) => format!("type {}", num),
                        Input::HangUp => "hang up".to_string(),
                        Input::PickUp => "pick up".to_string(),
                    }),
                    Symbol::Done(for_dur) => {
                        TransitionCause::Timeout((for_dur.as_millis() as f64) / 1000.0)
                    }
                },
                from: (*from).into(),
                to: (*to).into(),
            },
        }
    }
}

/// Describes states as part of an event in the `fernspielevt`
/// protocol.
#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct StateSummary {
    id: String,
}

impl<'a> From<&'a State> for StateSummary {
    fn from(state: &'a State) -> Self {
        StateSummary {
            id: state.id().to_string(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn convert_transition_event() {
        // given
        let internal_evt = MachineEvent::Transition {
            cause: Symbol::Dial(Input::pick_up()),
            from: &State::builder().id("1").build(),
            to: &State::builder().id("2").build(),
        };

        // when
        let public_event: FernspielEvent = From::from(&internal_evt);

        // then
        let expected_public_event = FernspielEvent::Transition {
            reason: TransitionCause::Dial("pick up".to_string()),
            from: StateSummary {
                id: "1".to_string(),
            },
            to: StateSummary {
                id: "2".to_string(),
            },
        };
        assert_eq!(public_event, expected_public_event)
    }

    #[test]
    fn generate_start_event_yaml() {
        // given
        let start_event = FernspielEvent::Start {
            initial: StateSummary {
                id: "1".to_string(),
            },
        };

        // when
        let serialized = serde_yaml::to_string(&start_event).unwrap();

        // then
        let expected_yaml = "---\n\
                             type: start\n\
                             initial:\n  \
                             id: \"1\"";

        assert_eq!(serialized, expected_yaml);
    }
}
