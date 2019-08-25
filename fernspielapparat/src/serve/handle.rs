use std::num::NonZeroU64;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct ConnectionHandle(NonZeroU64);

impl ConnectionHandle {
    pub fn generate() -> ConnectionHandleGenerator {
        ConnectionHandleGenerator(1)
    }
}

pub struct ConnectionHandleGenerator(u64);
impl Iterator for ConnectionHandleGenerator {
    type Item = ConnectionHandle;
    fn next(&mut self) -> Option<Self::Item> {
        let id = self.0;

        if id == 0 {
            // overflowed on last invocation or started at zero
            None
        } else {
            // did not overflowed yet, make a handle
            self.0 = self.0.wrapping_add(1); // pre-calculate next ID, overflowing to zero
            let id = unsafe {
                // ensured that non-zero above, call is safe
                NonZeroU64::new_unchecked(id)
            };
            Some(ConnectionHandle(id))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::u64::MAX;

    #[test]
    fn overflow() {
        // given
        let mut two_expected_before_overflow = ConnectionHandleGenerator(MAX - 1);

        // when
        let first = two_expected_before_overflow.next();
        let second = two_expected_before_overflow.next();
        let third = two_expected_before_overflow.next();
        let fourth = two_expected_before_overflow.next();

        // then
        assert_eq!(
            (first, second, third, fourth),
            (
                Some(ConnectionHandle(NonZeroU64::new(MAX - 1).unwrap())),
                Some(ConnectionHandle(NonZeroU64::new(MAX).unwrap())),
                None,
                None
            ),
            "Expected two handles before overflow"
        )
    }
}
