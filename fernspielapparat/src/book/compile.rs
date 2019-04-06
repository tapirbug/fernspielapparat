use crate::book::book;
use book::{Book, StateId, Transitions};
use crate::state::State;
use crate::sense::Input;
use failure::{Error, bail, format_err};
use std::time::Duration;

/// Compiles the phone book into states.
/// 
/// The initial state will be at index 0.
pub fn compile(mut book: Book) -> Result<Vec<State>, Error> {
    let defined_states = {
        let mut states : Vec<StateId> = book.states
            .keys()
            .map(Clone::clone)
            .collect();

        let initial_idx = states.iter()
            .position(|s| *s == book.initial)
            .ok_or(format_err!("Intitial state {:?} is undefined", book.initial))?;

        if initial_idx != 0 {
            states.swap(initial_idx, 0);
        }

        states
    };

    let any_transition = book.transitions.remove(&StateId::new("any"));
    let default_transition = Transitions::default();
    let default_state = book::State::default();

    defined_states.iter()
        .map(|id| {
            let state = book.states.get(id)
                    // defined_states are from the keys, unwrap of key access is safe
                    .unwrap()
                    .as_ref()
                    .unwrap_or(&default_state);

            let transitions = with_any(
                book.transitions.get(id)
                    .unwrap_or(&default_transition),
                any_transition.as_ref().unwrap_or(&default_transition)
            );

            let terminal = book.terminal
                .as_ref()
                .map(|terminal| *id == *terminal)
                .unwrap_or(false);

            compile_state(
                &defined_states,
                id,
                state,
                &transitions,
                terminal
            )
        })
        .collect()
}

fn compile_state(defined_states: &[StateId], state_id: &StateId, spec: &book::State, transitions: &Transitions, terminal: bool) -> Result<State, Error> {
    let mut state = State::builder()
        .name(format!("{}", state_id))
        .speech(spec.speech.clone())
        .terminal(terminal);
        // TODO speech_file

    if spec.ring != 0.0 {
        let ms = (spec.ring * 1000.0) as u64;
        state = state.ring_for(
            Duration::from_millis(ms)
        );
    }

    if let Some(ref timeout) = transitions.timeout {
        let ms = (timeout.after * 1000.0) as u64;

        let target_idx = defined_states.iter()
            .position(|id| *id == timeout.to)
            .ok_or_else(|| format_err!("Transition mentions unknown state: {:?}", timeout.to))?;

        state = state.timeout(Duration::from_millis(ms), target_idx)
    }

    // TODO done/end

    for (input, target_id) in transitions.dial.iter() {
        let target_idx = defined_states.iter()
            .position(|id| *id == *target_id)
            .ok_or_else(|| format_err!("Transition mentions unknown state: {:?}", target_id))?;

        if *input > 9 {
            bail!("Only digits in range 0--9 allowed for transitions, found: {}", input);
        }

        state = state.input(
            Input::digit(*input)?,
            target_idx
        );
    }

    if let Some(ref target_id) = transitions.hang_up {
        let target_idx = defined_states.iter()
            .position(|id| *id == *target_id)
            .ok_or_else(|| format_err!("Transition mentions unknown state: {:?}", target_id))?;

        state = state.input(Input::hang_up(), target_idx);
    }

    if let Some(ref target_id) = transitions.pick_up {
        let target_idx = defined_states.iter()
            .position(|id| *id == *target_id)
            .ok_or_else(|| format_err!("Transition mentions unknown state: {:?}", target_id))?;

        state = state.input(Input::pick_up(), target_idx);
    }

    Ok(state.build())
}

fn with_any(base: &Transitions, any: &Transitions) -> Transitions {
    let dial = base.dial.iter()
            .chain(any.dial.iter())
            .map(|(input, id)| (*input, id.clone()))
            .collect();

    let pick_up = base.pick_up.as_ref().or(any.pick_up.as_ref()).map(Clone::clone);
    let hang_up = base.hang_up.as_ref().or(any.hang_up.as_ref()).map(Clone::clone);
    let end = base.end.as_ref().or(any.end.as_ref()).map(Clone::clone);
    let timeout = base.timeout.as_ref().or(any.timeout.as_ref()).map(Clone::clone);

    Transitions {
        dial,
        pick_up,
        hang_up,
        end,
        timeout
    }
}
