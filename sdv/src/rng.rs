//! An implementation of .NET's Random Number Generator
//!
//! The algorithm orginates from p. 238 in Numerical Recipes in C 2nd Edition.
//! The method orginates from Knuth and is also know a "subtractive" generator.
//!
//! Two implementations were used as reference.
//! - [`MouseyPounds' Stardew Predictor`]
//! - [`Microsoft's .NET Reference Source`]
//!
//! Tested against MouseyPound's implementation to verify that it produces
//! identical results
//!
//!
//! [`MouseyPounds' Stardew Predictor`]: https://github.com/MouseyPounds/stardew-predictor/blob/master/cs-random.js
//! [`Microsoft's .NET Reference Source`]: http://referencesource.microsoft.com/#mscorlib/system/random.cs

use std::num::Wrapping;

use anyhow::{anyhow, Result};
use xxhash_rust::xxh32::Xxh32;

const SEED: Wrapping<i32> = Wrapping(161803398);
const SEED_ARRAY_LEN: usize = 56;

pub trait SeedGenerator {
    fn generate_seed(a: f64, b: f64, c: f64, d: f64, e: f64) -> i32;

    fn generate_day_save_seed(days_played: u32, game_id: u32, a: f64, b: f64, c: f64) -> i32 {
        Self::generate_seed(days_played as f64, (game_id / 2) as f64, a, b, c)
    }
}

pub struct LegacySeedGenerator {}
impl SeedGenerator for LegacySeedGenerator {
    fn generate_seed(a: f64, b: f64, c: f64, d: f64, e: f64) -> i32 {
        ((a % 2147483647.0
            + b % 2147483647.0
            + c % 2147483647.0
            + d % 2147483647.0
            + e % 2147483647.0)
            % 2147483647.0) as i32

        //            		return Game1.hash.GetDeterministicHashCode((int)(seedA % 2147483647.0), (int)(seedB % 2147483647.0), (int)(seedC % 2147483647.0), (int)(seedD % 2147483647.0), (int)(seedE % 2147483647.0));
    }
}

pub struct HashedSeedGenerator {}

impl HashedSeedGenerator {
    pub fn get_deterministic_hash_code(values: &[i32]) -> i32 {
        let mut hasher = Xxh32::new(0);

        for value in values {
            let data = value.to_le_bytes();
            hasher.update(&data);
        }
        hasher.digest() as i32
    }
}

impl SeedGenerator for HashedSeedGenerator {
    fn generate_seed(a: f64, b: f64, c: f64, d: f64, e: f64) -> i32 {
        Self::get_deterministic_hash_code(&[
            (a % 2147483647.0) as i32,
            (b % 2147483647.0) as i32,
            (c % 2147483647.0) as i32,
            (d % 2147483647.0) as i32,
            (e % 2147483647.0) as i32,
        ])
    }
}

/// .NET Random Number Generator
pub struct Rng {
    next: usize,
    next_p: usize,
    seed_array: [Wrapping<i32>; SEED_ARRAY_LEN],
}

impl Rng {
    /// Create and new [Rng].
    pub fn new(seed: i32) -> Rng {
        let seed = Wrapping(seed);
        let subtraction = if seed == Wrapping(i32::MIN) {
            Wrapping(i32::MAX)
        } else {
            Wrapping(seed.0.abs())
        };
        let mut m_j = SEED - subtraction;
        let mut seed_array = [Wrapping(0i32); SEED_ARRAY_LEN];
        seed_array[SEED_ARRAY_LEN - 1] = m_j;
        let mut m_k = Wrapping(1);

        for i in 1..(SEED_ARRAY_LEN - 1) {
            let ii = (21 * i) % (SEED_ARRAY_LEN - 1);
            seed_array[ii] = m_k;
            m_k = m_j - m_k;
            if m_k < Wrapping(0) {
                m_k += i32::MAX;
            }
            m_j = seed_array[ii];
        }

        for _ in 1..5 {
            for i in 1..SEED_ARRAY_LEN {
                seed_array[i] -= seed_array[1 + (i + 30) % (SEED_ARRAY_LEN - 1)];
                if seed_array[i] < Wrapping(0) {
                    seed_array[i] += i32::MAX;
                }
            }
        }

        Rng {
            next: 0,
            next_p: 21,
            seed_array,
        }
    }

    fn internal_sample(&mut self) -> i32 {
        let mut next = self.next + 1;
        let mut next_p = self.next_p + 1;
        if next >= SEED_ARRAY_LEN {
            next = 1
        }
        if next_p >= SEED_ARRAY_LEN {
            next_p = 1
        }

        let mut val = self.seed_array[next] - self.seed_array[next_p];

        if val == Wrapping(i32::MAX) {
            val -= 1;
        }
        if val < Wrapping(0) {
            val += i32::MAX;
        }

        self.seed_array[next] = val;

        self.next = next;
        self.next_p = next_p;

        val.0
    }

    /// Pull a floating point sample from the [Rng].
    ///
    /// Returned values will be between 0.0 and 1.0.  Value will
    /// Have 31 bits of "entropy".
    pub fn sample(&mut self) -> f64 {
        self.internal_sample() as f64 / i32::MAX as f64
    }

    /// Pull a floating point sample from the [Rng].
    ///
    /// Returned values will be between 0.0 and 1.0.  Value will
    /// Have 32 bits of "entropy".
    pub fn sample_large_range(&mut self) -> f64 {
        let mut result = self.internal_sample();
        let negative = self.internal_sample() % 2 == 0;
        if negative {
            result = -result;
        }
        let mut result = result as f64;
        result += (i32::MAX - 1) as f64;
        result /= ((2 * (i32::MAX as u32)) - 1) as f64;
        result
    }

    /// Pull the next i32 sample from the [Rng]
    ///
    /// Returned values will be between 0 and i32::MAX.
    pub fn next_i32(&mut self) -> i32 {
        self.internal_sample()
    }

    /// Pull a floating point sample from the [Rng].
    ///
    /// Returned values will be between 0.0 and 1.0, inclusive.  Value will
    /// Have 31 bits of "entropy".
    pub fn next_double(&mut self) -> f64 {
        self.sample()
    }

    /// Pull a boolean sample from the [Rng].
    ///
    pub fn next_bool(&mut self) -> bool {
        self.next_double() < 0.5
    }

    /// Pull a weighted boolean sample from the [Rng].
    ///
    /// `chance` ([0.0..1.0]) is the probablity that true will be returned.
    pub fn next_weighted_bool(&mut self, chance: f64) -> bool {
        if chance >= 1.0 {
            return true;
        }
        self.next_double() < chance
    }

    /// Pull a value with maximum value from the [Rng].
    ///
    /// Value returned is in the range [0, max_val).
    pub fn next_max(&mut self, max_val: i32) -> i32 {
        (self.sample() * max_val as f64) as i32
    }

    /// Pull a value in the range the [Rng]
    ///
    /// Value returned is in the range [min_val, max_val).
    pub fn next_range(&mut self, min_val: i32, max_val: i32) -> Result<i32> {
        if min_val > max_val {
            return Err(anyhow!(
                "max_val ({}) must be larger than min_val ({})",
                max_val,
                min_val
            ));
        }

        let min_val = min_val as i64;
        let max_val = max_val as i64;
        let range = max_val - min_val;

        if range <= i32::MAX as i64 {
            Ok((self.sample() * range as f64) as i32 + min_val as i32)
        } else {
            Ok(((self.sample_large_range() * range as f64) as i64 + min_val) as i32)
        }
    }

    pub fn chooose_from<'a, T>(&mut self, choices: &'a [T]) -> &'a T {
        &choices[self.next_max(choices.len() as i32) as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next() {
        // Test values were generated using MouseyPound's JS implementation.
        let mut rng = Rng::new(34 + 327349652 / 2);
        assert_eq!(rng.next_i32(), 1903971056);
        assert_eq!(rng.next_i32(), 2089011827);
        assert_eq!(rng.next_i32(), 539281092);
        assert_eq!(rng.next_i32(), 729551037);
    }

    #[test]
    fn next_range() {
        // Test values were generated using MouseyPound's JS implementation.
        let mut rng = Rng::new(34 + 327349652 / 2);
        assert_eq!(rng.next_range(1, 10).unwrap(), 8);
        assert_eq!(rng.next_range(1, 10).unwrap(), 9);
        assert_eq!(rng.next_range(1, 10).unwrap(), 3);
        assert_eq!(rng.next_range(1, 10).unwrap(), 4);
    }
}
