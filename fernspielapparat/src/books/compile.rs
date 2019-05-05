use crate::books::book;
use crate::senses::Input;
use crate::states::{State, StateBuilder};
use book::{Book, StateId, Transitions};
use failure::{bail, format_err, Error};
use std::path::PathBuf;
use std::time::Duration;

/// Compiles the phone book into states.
///
/// The initial state will be at index 0.
pub fn compile(mut book: Book) -> Result<Vec<State>, Error> {
    let defined_states = {
        let mut states: Vec<StateId> = book.states.keys().map(Clone::clone).collect();

        let initial_idx = states
            .iter()
            .position(|s| *s == book.initial)
            .ok_or_else(|| format_err!("Intitial state {:?} is undefined", book.initial))?;

        if initial_idx != 0 {
            states.swap(initial_idx, 0);
        }

        states
    };

    let any_transition = book.transitions.remove(&StateId::new("any"));
    let default_transition = Transitions::default();
    let default_state = book::State::default();

    defined_states
        .iter()
        .map(|id| {
            let state = book
                .states
                .get(id)
                // defined_states are from the keys, unwrap of key access is safe
                .unwrap()
                .as_ref()
                .unwrap_or(&default_state);

            let transitions = with_any(
                book.transitions.get(id).unwrap_or(&default_transition),
                any_transition.as_ref().unwrap_or(&default_transition),
            );

            let terminal = book
                .terminal
                .as_ref()
                .map(|terminal| *id == *terminal)
                .unwrap_or(false);

            compile_state(&defined_states, id, state, &transitions, terminal)
        })
        .collect()
}

fn compile_state(
    defined_states: &[StateId],
    state_id: &StateId,
    spec: &book::State,
    transitions: &Transitions,
    terminal: bool,
) -> Result<State, Error> {
    let mut state = State::builder()
        .name(if spec.name.is_empty() {
            format!("{}", state_id)
        } else {
            spec.name.clone()
        })
        .speech(spec.speech.clone())
        .terminal(terminal)
        .content_files(spec.content.iter().map(PathBuf::from))
        .environment_files(spec.content.iter().map(PathBuf::from));

    state = compile_ring(state, spec.ring);

    if let Some(ref timeout) = transitions.timeout {
        state = lookup_state(defined_states, &timeout.to)
            .map(|idx| compile_timeout(state, timeout.after, idx))?
    }

    for (dial_pattern, target_id) in transitions.dial.iter() {
        let mut pattern_digits = dial_pattern.chars().filter(|c| *c >= '0' && *c <= '9');
        let input = pattern_digits
            .next()
            .ok_or_else(|| format_err!("Pattern contained no digits: \"{}\"", dial_pattern))
            .map(|c| (c as i32) - ('0' as i32))?;

        if pattern_digits.next().is_some() {
            bail!(
                "Pattern can currently only consist of a single digit, but got: \"{}\"",
                dial_pattern
            );
        }

        let target_idx = lookup_state(defined_states, target_id)?;

        state = state.input(Input::digit(input)?, target_idx);
    }

    if let Some(ref target_id) = transitions.hang_up {
        let target_idx = lookup_state(defined_states, target_id)?;
        state = state.input(Input::hang_up(), target_idx);
    }

    if let Some(ref target_id) = transitions.pick_up {
        let target_idx = lookup_state(defined_states, target_id)?;
        state = state.input(Input::pick_up(), target_idx);
    }

    if let Some(ref target_id) = transitions.end {
        let target_idx = lookup_state(defined_states, target_id)?;
        state = state.end(target_idx);
    }

    Ok(state.build())
}

fn lookup_state(defined_states: &[StateId], search_id: &StateId) -> Result<usize, Error> {
    defined_states
        .iter()
        .position(|id| id == search_id)
        .ok_or_else(|| format_err!("Transition mentions unknown state: {}", search_id))
}

fn compile_ring(state: StateBuilder, ring: f64) -> StateBuilder {
    if ring == 0.0 {
        state
    } else {
        let ms = (ring * 1000.0) as u64;
        state.ring_for(Duration::from_millis(ms))
    }
}

fn compile_timeout(state: StateBuilder, after: f64, to: usize) -> StateBuilder {
    let ms = (after * 1000.0) as u64;
    state.timeout(Duration::from_millis(ms), to)
}

fn with_any(base: &Transitions, any: &Transitions) -> Transitions {
    let dial = base
        .dial
        .iter()
        .chain(any.dial.iter())
        .map(|(input, id)| (input.clone(), id.clone()))
        .collect();

    let pick_up = base
        .pick_up
        .as_ref()
        .or_else(|| any.pick_up.as_ref())
        .map(Clone::clone);
    let hang_up = base
        .hang_up
        .as_ref()
        .or_else(|| any.hang_up.as_ref())
        .map(Clone::clone);
    let end = base
        .end
        .as_ref()
        .or_else(|| any.end.as_ref())
        .map(Clone::clone);
    let timeout = base
        .timeout
        .as_ref()
        .or_else(|| any.timeout.as_ref())
        .map(Clone::clone);

    Transitions {
        dial,
        pick_up,
        hang_up,
        end,
        timeout,
    }
}
