// Direct behavioral port of diffmatchpatch/match.go from sergi/go-diff v1.4.0.

use crate::DiffMatchPatch;
use crate::util::{index_of, last_index_of};
use std::collections::HashMap;

impl DiffMatchPatch {
    /// Locate the best instance of `pattern` in `text` near a byte offset.
    #[must_use]
    pub fn match_main<A: AsRef<[u8]>, B: AsRef<[u8]>>(
        &self,
        text: A,
        pattern: B,
        location: isize,
    ) -> isize {
        let text = text.as_ref();
        let pattern = pattern.as_ref();
        let location = location.clamp(0, text.len() as isize);
        if text == pattern {
            return 0;
        }
        if text.is_empty() {
            return -1;
        }
        let location_usize = location as usize;
        if location_usize + pattern.len() <= text.len()
            && text[location_usize..location_usize + pattern.len()] == *pattern
        {
            return location;
        }
        self.match_bitap(text, pattern, location)
    }

    /// Locate a fuzzy match using the Bitap algorithm.
    #[must_use]
    pub fn match_bitap<A: AsRef<[u8]>, B: AsRef<[u8]>>(
        &self,
        text: A,
        pattern: B,
        location: isize,
    ) -> isize {
        let text = text.as_ref();
        let pattern = pattern.as_ref();
        let alphabet = self.match_alphabet(pattern);
        let mut score_threshold = self.match_threshold;
        let mut best_location = index_of(text, pattern, location);
        if best_location != -1 {
            score_threshold = score_threshold.min(self.match_bitap_score(
                0,
                best_location,
                location,
                pattern.len(),
            ));
            best_location = last_index_of(text, pattern, location + pattern.len() as isize);
            if best_location != -1 {
                score_threshold = score_threshold.min(self.match_bitap_score(
                    0,
                    best_location,
                    location,
                    pattern.len(),
                ));
            }
        }

        let match_mask = shift_one(pattern.len().wrapping_sub(1));
        best_location = -1;
        let mut bin_max = pattern.len() as isize + text.len() as isize;
        let mut last_rd: Vec<isize> = Vec::new();
        for errors in 0..pattern.len() {
            let mut bin_min = 0_isize;
            let mut bin_mid = bin_max;
            while bin_min < bin_mid {
                if self.match_bitap_score(errors, location + bin_mid, location, pattern.len())
                    <= score_threshold
                {
                    bin_min = bin_mid;
                } else {
                    bin_max = bin_mid;
                }
                bin_mid = (bin_max - bin_min) / 2 + bin_min;
            }
            bin_max = bin_mid;
            let mut start = (location - bin_mid + 1).max(1);
            let finish = (location + bin_mid).min(text.len() as isize) + pattern.len() as isize;
            let mut rd = vec![0_isize; finish as usize + 2];
            rd[finish as usize + 1] = shift_one(errors).wrapping_sub(1);
            let mut index = finish;
            while index >= start {
                let char_match = if index > text.len() as isize {
                    0
                } else {
                    *alphabet.get(&text[(index - 1) as usize]).unwrap_or(&0)
                };
                let current = index as usize;
                if errors == 0 {
                    rd[current] = (rd[current + 1].wrapping_shl(1) | 1) & char_match;
                } else {
                    rd[current] = ((rd[current + 1].wrapping_shl(1) | 1) & char_match)
                        | (((last_rd[current + 1] | last_rd[current]).wrapping_shl(1)) | 1)
                        | last_rd[current + 1];
                }
                if rd[current] & match_mask != 0 {
                    let score = self.match_bitap_score(errors, index - 1, location, pattern.len());
                    if score <= score_threshold {
                        score_threshold = score;
                        best_location = index - 1;
                        if best_location > location {
                            start = (2 * location - best_location).max(1);
                        } else {
                            break;
                        }
                    }
                }
                index -= 1;
            }
            if self.match_bitap_score(errors + 1, location, location, pattern.len())
                > score_threshold
            {
                break;
            }
            last_rd = rd;
        }
        best_location
    }

    fn match_bitap_score(
        &self,
        errors: usize,
        found: isize,
        location: isize,
        pattern_len: usize,
    ) -> f64 {
        let accuracy = errors as f64 / pattern_len as f64;
        let proximity = (location - found).unsigned_abs() as f64;
        if self.match_distance == 0 {
            if proximity == 0.0 { accuracy } else { 1.0 }
        } else {
            accuracy + proximity / self.match_distance as f64
        }
    }

    /// Initialize the Bitap alphabet using byte-oriented masks.
    #[must_use]
    pub fn match_alphabet<A: AsRef<[u8]>>(&self, pattern: A) -> HashMap<u8, isize> {
        let pattern = pattern.as_ref();
        let mut alphabet = HashMap::new();
        for &byte in pattern {
            alphabet.entry(byte).or_insert(0_isize);
        }
        for (index, &byte) in pattern.iter().enumerate() {
            let value = alphabet[&byte] | shift_one(pattern.len() - index - 1);
            alphabet.insert(byte, value);
        }
        alphabet
    }
}

fn shift_one(amount: usize) -> isize {
    1_isize.checked_shl(amount as u32).unwrap_or(0)
}
