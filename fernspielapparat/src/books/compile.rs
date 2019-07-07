use crate::books::spec;
use crate::senses::Input;
use crate::states::{State, StateBuilder};
pub use book::Book;
use failure::{bail, format_err, Error};
use log::warn;
use spec::{Id, Transitions};
use std::collections::HashMap;
use std::time::Duration;

mod book {
    use crate::acts::SoundSpec;
    use crate::books::spec;
    use crate::states::State;
    use failure::{format_err, Error};
    use log::{debug, warn};
    use std::cmp::min;
    use std::collections::hash_map::DefaultHasher;
    use std::fs::write;
    use std::hash::Hasher;
    use std::path::{Path, PathBuf};
    use tavla::{any_voice, Speech, Voice};
    use tempfile::{tempdir, TempDir};

    const KIB: usize = 1024;

    pub struct Book {
        pub(crate) states: Vec<State>,
        sounds: Vec<SoundSpec>,
        /// Get deleted when book is destroyed
        compiled_speech_dir: TempDir,
    }

    impl Book {
        pub fn builder() -> Result<BookBuilder, Error> {
            let builder = BookBuilder {
                book: Book {
                    states: vec![],
                    sounds: vec![],
                    compiled_speech_dir: tempdir()?,
                },
            };
            Ok(builder)
        }

        pub fn states(&self) -> &[State] {
            &self.states
        }

        pub fn sounds(&self) -> &[SoundSpec] {
            &self.sounds
        }
    }

    pub struct BookBuilder {
        book: Book,
    }

    impl BookBuilder {
        /// No more than 256KiB of text are allowed for synthesis.
        const MAX_TEXT_LEN: usize = 256 * KIB;
        /// Maximum amount of spoken text characters to include in
        /// generated filenames.
        const MAX_SUMMARY_LEN: usize = 60;

        pub fn state(&mut self, state: State) -> &mut Self {
            self.book.states.push(state);
            self
        }

        /// If the given sound spec describes text-to-speech, adds a
        /// temporary file to the books temporary directory with the
        /// speech content.
        ///
        /// The content file is then set to the given spec and its
        /// speech text is removed.spec
        fn prepare_sound(sound: &mut spec::Sound, cache_directory: &Path) -> Result<(), Error> {
            if let Some(mut text) = sound.speech.take() {
                if text.len() > Self::MAX_TEXT_LEN {
                    shrink_to_max(&mut text, Self::MAX_TEXT_LEN);
                }

                let mut hash = DefaultHasher::new();
                hash.write(text.as_bytes());
                let hash = hash.finish();

                // work on a slice of the maximum summary length
                // in case there are no whitespaces.
                let summary = summarize(&text, Self::MAX_SUMMARY_LEN);

                let mut filename = PathBuf::from(cache_directory);
                filename.push(format!(
                    "{hash}-{summary}.wav",
                    hash = hash,
                    summary = summary
                ));

                debug!("Preparing speech {:?}...", &filename);
                debug!("Text: {:?}", text);
                let voice = any_voice()?;
                voice.speak_to_file(text, &filename)?.await_done()?;

                sound.file = filename.to_str().unwrap().into();
            }

            match Self::prepare_data_uri(&sound.file, cache_directory) {
                Ok(Some(persisted_data_uri_path)) => {
                    sound.file = persisted_data_uri_path.to_str().unwrap().into()
                }
                Ok(None) => (),
                Err(err) => return Err(err),
            };

            Ok(())
        }

        fn prepare_data_uri(
            potential_data_uri: &str,
            cache_directory: &Path,
        ) -> Result<Option<PathBuf>, Error> {
            use base64::decode;

            if potential_data_uri.starts_with("data:") {
                let rest = &potential_data_uri["data:".len()..];
                let mime_end = rest[0..rest.len().min(32)]
                    .find(";base64,")
                    .ok_or_else(|| format_err!("Data uri was not base64"))?;
                let mime = &rest[0..mime_end];
                let content = decode(&rest[(mime_end + ";base64,".len())..].trim())?;

                let mut hash = DefaultHasher::new();
                hash.write(&content);
                let extension = match mime {
                    "audio/mpeg" | "audio/mp3" | "audio/mpeg3" | "audio/x-mpeg-3"
                    | "video/mpeg" | "video/x-mpeg" => "mp3",
                    _ => "wav",
                };
                let mut path = PathBuf::from(cache_directory);
                path.push(format!(
                    "{name}.{extension}",
                    name = hash.finish(),
                    extension = extension
                ));
                debug!("Writing base64 encoded {:?}", path);

                write(&path, &content)?;
                Ok(Some(path))
            } else {
                Ok(None)
            }
        }

        pub fn sound(&mut self, mut sound: spec::Sound) -> Result<&mut Self, Error> {
            let cache_directory: &Path = self.book.compiled_speech_dir.as_ref();
            Self::prepare_sound(&mut sound, cache_directory)?;
            let path = sound.file.clone();

            self.book.sounds.push({
                let mut builder = SoundSpec::builder().source(path);

                if let Some(offset) = sound.start_offset {
                    builder.start_offset(offset)?;
                }

                if let Some(backoff) = sound.backoff {
                    builder.backoff(backoff)?;
                }

                builder.looping(sound.looping).build()
            });

            Ok(self)
        }

        pub fn build(self) -> Book {
            self.book
        }
    }

    fn shrink_to_max(text: &mut String, max: usize) {
        warn!(
            "Sound text has a size of {actual}KiB, \
             which exceeds the maximum of {max}KiB \
             by {excess}KiB. \
             Text is cut off after the maximum size.",
            actual = text.len() / KIB,
            max = max / KIB,
            excess = (text.len() - max) / KIB
        );
        text.replace_range(next_char_boundary(&text, max).., "");
    }

    fn next_char_boundary(string: &str, search_start: usize) -> usize {
        if search_start >= string.len() {
            string.len()
        } else {
            (search_start..string.len())
                .find(|i| string.is_char_boundary(*i))
                .unwrap_or_else(|| string.len())
        }
    }

    /// Summary of text for inclusion in a filename or URL,
    /// with only alphanumeric ascii letters and joining
    /// hyphens.
    fn summarize(text: &str, max_summary_len: usize) -> String {
        text[0..min(text.len(), max_summary_len)]
            // take a maximum of five words
            .split_whitespace()
            .take(5)
            .map(|s| {
                s.chars()
                    // filter out characters that could make
                    // problems in filenames and any incomplete
                    // utf-8 sequence at the end.
                    // (sorry if you are debugging a chinese
                    // phonebook)
                    .filter(char::is_ascii_alphanumeric)
                    // and normalize them for case-insenitive
                    // vs. case-sensitive filesystems
                    .map(|c| c.to_ascii_lowercase())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("-")
    }

    #[cfg(test)]
    mod test {
        use super::*;
        use crate::books::file::load;
        use crate::books::spec::Id;
        use std::fs::read_dir;
        use tempfile::tempdir;

        #[test]
        fn prepare_wav_files_from_default_yaml() {
            // given
            let tempdir = tempdir().expect("could not create temporary directory");

            // when
            let mut petrov_book = load("./resources/demo.yaml").unwrap();
            let missiles_launched_opt = petrov_book.sounds.get_mut(&Id::new("missiles_launched"));
            match missiles_launched_opt {
                Some(sound_spec) => {
                    assert!(sound_spec.speech.is_some());
                    BookBuilder::prepare_sound(sound_spec, tempdir.path()).unwrap();
                }
                _ => panic!("Could not load demo file"),
            }
            let temp_contents = read_dir(tempdir.path()).unwrap();

            // then
            assert_eq!(
                temp_contents.count(),
                1,
                "Expected exactly one generated file."
            );
        }
    }
}

/// Compiles the phone book into states and sounds.
///
/// This also prepares espeak speech into WAV files
/// in a temporary directory.
pub fn compile(book: spec::Book) -> Result<Book, Error> {
    let mut builder = Book::builder()?;

    let spec::Book {
        states,
        sounds,
        initial,
        mut transitions,
    } = book;

    let sounds: HashMap<Id, usize> = sounds
        .into_iter()
        .enumerate()
        .map(|(idx, (id, s))| builder.sound(s).map(|_| (id, idx)))
        .collect::<Result<_, Error>>()?;

    let defined_states = {
        let mut states: Vec<Id> = states.keys().map(Clone::clone).collect();

        let initial_idx = states
            .iter()
            .position(|s| *s == initial)
            .ok_or_else(|| format_err!("Intitial state {:?} is undefined", initial))?;

        if initial_idx != 0 {
            states.swap(initial_idx, 0);
        }

        states
    };

    let any_transition = transitions.remove(&Id::new("any"));
    let default_transition = Transitions::default();
    let default_state = spec::State::default();

    defined_states
        .iter()
        .map(|id| {
            let state = states
                .get(id)
                // defined_states are from the keys, unwrap of key access is safe
                .unwrap()
                .as_ref()
                .unwrap_or(&default_state);

            let transitions = with_any(
                transitions.get(id).unwrap_or(&default_transition),
                any_transition.as_ref().unwrap_or(&default_transition),
            );

            let state = compile_state(&defined_states, id, state, &transitions, &sounds)?;
            builder.state(state);
            Ok(())
        })
        .collect::<Result<Vec<()>, Error>>()?;

    Ok(builder.build())
}

fn compile_state(
    defined_states: &[Id],
    state_id: &Id,
    spec: &spec::State,
    transitions: &Transitions,
    sounds: &HashMap<Id, usize>,
) -> Result<State, Error> {
    let mut state = State::builder()
        .name(if spec.name.is_empty() {
            format!("{}", state_id)
        } else {
            spec.name.clone()
        })
        .terminal(spec.terminal);

    state = state.sounds(
        spec.sounds
            .iter()
            .map(|id| {
                if sounds.contains_key(id) {
                    Ok(sounds[id])
                } else {
                    bail!("State {:?} uses undefined Sound ID {:?}", state_id, id)
                }
            })
            .collect::<Result<Vec<usize>, Error>>()?,
    );

    if !spec.speech.is_empty() {
        warn!("speech on a state is deprecated and should not be used in new phonebooks. Use a sound instead.");
        state = state.speech(spec.speech.clone())
    }

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

fn lookup_state(defined_states: &[Id], search_id: &Id) -> Result<usize, Error> {
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
