// Direct behavioral port of diffmatchpatch/diff.go from sergi/go-diff v1.4.0.

use crate::util::{
    common_prefix_runes, common_suffix_runes, decode_go_rune, decode_go_runes, encode_go_runes,
    escaped_for_patch, find_runes, find_subslice, html_escape, index_of_runes, indexes_to_string,
    query_unescape, rune_count, rune_to_int, split_bytes, string_to_indexes,
};
use crate::{
    DIFF_DELETE, DIFF_EQUAL, DIFF_INSERT, Deadline, Diff, DiffMatchPatch, Operation, PatchError,
    Text,
};
use std::collections::HashMap;
use std::time::Instant;

impl DiffMatchPatch {
    /// Find the differences between two byte strings.
    /// Invalid UTF-8 bytes are replaced exactly as Go's `[]rune(string)` conversion does.
    #[must_use]
    pub fn diff_main<A: AsRef<[u8]>, B: AsRef<[u8]>>(
        &self,
        text1: A,
        text2: B,
        check_lines: bool,
    ) -> Vec<Diff> {
        self.diff_main_runes(
            &decode_go_runes(text1.as_ref()),
            &decode_go_runes(text2.as_ref()),
            check_lines,
        )
    }

    /// Find the differences between two Go-rune sequences.
    #[must_use]
    pub fn diff_main_runes(&self, text1: &[u32], text2: &[u32], check_lines: bool) -> Vec<Diff> {
        let deadline = if self.diff_timeout.is_zero() {
            Deadline::Infinite
        } else {
            Deadline::At(Instant::now() + self.diff_timeout)
        };
        self.diff_main_runes_deadline(text1, text2, check_lines, deadline)
    }

    fn diff_main_runes_deadline(
        &self,
        text1: &[u32],
        text2: &[u32],
        check_lines: bool,
        deadline: Deadline,
    ) -> Vec<Diff> {
        if text1 == text2 {
            if text1.is_empty() {
                return Vec::new();
            }
            return vec![Diff::new(DIFF_EQUAL, encode_go_runes(text1))];
        }

        let common_prefix_length = common_prefix_runes(text1, text2);
        let common_prefix = &text1[..common_prefix_length];
        let mut middle1 = &text1[common_prefix_length..];
        let mut middle2 = &text2[common_prefix_length..];

        let common_suffix_length = common_suffix_runes(middle1, middle2);
        let common_suffix = &middle1[middle1.len() - common_suffix_length..];
        middle1 = &middle1[..middle1.len() - common_suffix_length];
        middle2 = &middle2[..middle2.len() - common_suffix_length];

        let mut diffs = self.diff_compute(middle1, middle2, check_lines, deadline);
        if !common_prefix.is_empty() {
            diffs.insert(0, Diff::new(DIFF_EQUAL, encode_go_runes(common_prefix)));
        }
        if !common_suffix.is_empty() {
            diffs.push(Diff::new(DIFF_EQUAL, encode_go_runes(common_suffix)));
        }
        self.diff_cleanup_merge(diffs)
    }

    fn diff_compute(
        &self,
        text1: &[u32],
        text2: &[u32],
        check_lines: bool,
        deadline: Deadline,
    ) -> Vec<Diff> {
        if text1.is_empty() {
            return vec![Diff::new(DIFF_INSERT, encode_go_runes(text2))];
        }
        if text2.is_empty() {
            return vec![Diff::new(DIFF_DELETE, encode_go_runes(text1))];
        }

        let (long_text, short_text, operation) = if text1.len() > text2.len() {
            (text1, text2, DIFF_DELETE)
        } else {
            (text2, text1, DIFF_INSERT)
        };
        if let Some(index) = find_runes(long_text, short_text) {
            return vec![
                Diff::new(operation, encode_go_runes(&long_text[..index])),
                Diff::new(DIFF_EQUAL, encode_go_runes(short_text)),
                Diff::new(
                    operation,
                    encode_go_runes(&long_text[index + short_text.len()..]),
                ),
            ];
        }
        if short_text.len() == 1 {
            return vec![
                Diff::new(DIFF_DELETE, encode_go_runes(text1)),
                Diff::new(DIFF_INSERT, encode_go_runes(text2)),
            ];
        }

        if let Some(half_match) = self.diff_half_match_runes(text1, text2) {
            let mut diffs = self.diff_main_runes_deadline(
                &half_match[0],
                &half_match[2],
                check_lines,
                deadline,
            );
            diffs.push(Diff::new(DIFF_EQUAL, encode_go_runes(&half_match[4])));
            diffs.extend(self.diff_main_runes_deadline(
                &half_match[1],
                &half_match[3],
                check_lines,
                deadline,
            ));
            return diffs;
        }
        if check_lines && text1.len() > 100 && text2.len() > 100 {
            return self.diff_line_mode(text1, text2, deadline);
        }
        self.diff_bisect_runes(text1, text2, deadline)
    }

    fn diff_line_mode(&self, text1: &[u32], text2: &[u32], deadline: Deadline) -> Vec<Diff> {
        let (encoded1, encoded2, lines) =
            self.diff_lines_to_runes(encode_go_runes(text1), encode_go_runes(text2));
        let mut diffs = self.diff_main_runes_deadline(&encoded1, &encoded2, false, deadline);
        diffs = self.diff_chars_to_lines(&diffs, &lines);
        diffs = self.diff_cleanup_semantic(diffs);
        diffs.push(Diff::new(DIFF_EQUAL, b""));

        let mut pointer = 0_usize;
        let mut count_delete = 0_usize;
        let mut count_insert = 0_usize;
        let mut text_delete = Vec::new();
        let mut text_insert = Vec::new();
        while pointer < diffs.len() {
            match diffs[pointer].operation {
                DIFF_INSERT => {
                    count_insert += 1;
                    text_insert.extend_from_slice(diffs[pointer].text.as_bytes());
                }
                DIFF_DELETE => {
                    count_delete += 1;
                    text_delete.extend_from_slice(diffs[pointer].text.as_bytes());
                }
                DIFF_EQUAL => {
                    if count_delete >= 1 && count_insert >= 1 {
                        let replaced = count_delete + count_insert;
                        let start = pointer - replaced;
                        let end = start + replaced;
                        diffs.splice(start..end, std::iter::empty());
                        pointer = start;
                        let replacements = self.diff_main_runes_deadline(
                            &decode_go_runes(&text_delete),
                            &decode_go_runes(&text_insert),
                            false,
                            deadline,
                        );
                        let replacement_count = replacements.len();
                        for replacement in replacements.into_iter().rev() {
                            diffs.insert(pointer, replacement);
                        }
                        pointer += replacement_count;
                    }
                    count_insert = 0;
                    count_delete = 0;
                    text_delete.clear();
                    text_insert.clear();
                }
                _ => {}
            }
            pointer += 1;
        }
        diffs.pop();
        diffs
    }

    /// Myers bisect diff with an explicit deadline.
    #[must_use]
    pub fn diff_bisect<A: AsRef<[u8]>, B: AsRef<[u8]>>(
        &self,
        text1: A,
        text2: B,
        deadline: Deadline,
    ) -> Vec<Diff> {
        self.diff_bisect_runes(
            &decode_go_runes(text1.as_ref()),
            &decode_go_runes(text2.as_ref()),
            deadline,
        )
    }

    fn diff_bisect_runes(&self, text1: &[u32], text2: &[u32], deadline: Deadline) -> Vec<Diff> {
        let text1_len = text1.len() as isize;
        let text2_len = text2.len() as isize;
        let max_d = (text1_len + text2_len + 1) / 2;
        let v_offset = max_d;
        let v_length = 2 * max_d;
        let mut v1 = vec![-1_isize; v_length as usize];
        let mut v2 = vec![-1_isize; v_length as usize];
        if v_offset + 1 < v_length {
            v1[(v_offset + 1) as usize] = 0;
            v2[(v_offset + 1) as usize] = 0;
        }
        let delta = text1_len - text2_len;
        let front = delta % 2 != 0;
        let mut k1_start = 0;
        let mut k1_end = 0;
        let mut k2_start = 0;
        let mut k2_end = 0;

        for d in 0..max_d {
            if d % 16 == 0 && deadline.expired() {
                break;
            }

            let mut k1 = -d + k1_start;
            while k1 <= d - k1_end {
                let k1_offset = v_offset + k1;
                let mut x1 = if k1 == -d
                    || (k1 != d && v1[(k1_offset - 1) as usize] < v1[(k1_offset + 1) as usize])
                {
                    v1[(k1_offset + 1) as usize]
                } else {
                    v1[(k1_offset - 1) as usize] + 1
                };
                let mut y1 = x1 - k1;
                while x1 < text1_len && y1 < text2_len && text1[x1 as usize] == text2[y1 as usize] {
                    x1 += 1;
                    y1 += 1;
                }
                v1[k1_offset as usize] = x1;
                if x1 > text1_len {
                    k1_end += 2;
                } else if y1 > text2_len {
                    k1_start += 2;
                } else if front {
                    let k2_offset = v_offset + delta - k1;
                    if k2_offset >= 0 && k2_offset < v_length && v2[k2_offset as usize] != -1 {
                        let x2 = text1_len - v2[k2_offset as usize];
                        if x1 >= x2 {
                            return self.diff_bisect_split(
                                text1,
                                text2,
                                x1 as usize,
                                y1 as usize,
                                deadline,
                            );
                        }
                    }
                }
                k1 += 2;
            }

            let mut k2 = -d + k2_start;
            while k2 <= d - k2_end {
                let k2_offset = v_offset + k2;
                let mut x2 = if k2 == -d
                    || (k2 != d && v2[(k2_offset - 1) as usize] < v2[(k2_offset + 1) as usize])
                {
                    v2[(k2_offset + 1) as usize]
                } else {
                    v2[(k2_offset - 1) as usize] + 1
                };
                let mut y2 = x2 - k2;
                while x2 < text1_len
                    && y2 < text2_len
                    && text1[(text1_len - x2 - 1) as usize] == text2[(text2_len - y2 - 1) as usize]
                {
                    x2 += 1;
                    y2 += 1;
                }
                v2[k2_offset as usize] = x2;
                if x2 > text1_len {
                    k2_end += 2;
                } else if y2 > text2_len {
                    k2_start += 2;
                } else if !front {
                    let k1_offset = v_offset + delta - k2;
                    if k1_offset >= 0 && k1_offset < v_length && v1[k1_offset as usize] != -1 {
                        let x1 = v1[k1_offset as usize];
                        let y1 = v_offset + x1 - k1_offset;
                        let mirrored_x2 = text1_len - x2;
                        if x1 >= mirrored_x2 {
                            return self.diff_bisect_split(
                                text1,
                                text2,
                                x1 as usize,
                                y1 as usize,
                                deadline,
                            );
                        }
                    }
                }
                k2 += 2;
            }
        }

        vec![
            Diff::new(DIFF_DELETE, encode_go_runes(text1)),
            Diff::new(DIFF_INSERT, encode_go_runes(text2)),
        ]
    }

    pub(crate) fn diff_bisect_split(
        &self,
        text1: &[u32],
        text2: &[u32],
        x: usize,
        y: usize,
        deadline: Deadline,
    ) -> Vec<Diff> {
        let mut diffs = self.diff_main_runes_deadline(&text1[..x], &text2[..y], false, deadline);
        diffs.extend(self.diff_main_runes_deadline(&text1[x..], &text2[y..], false, deadline));
        diffs
    }

    /// Split two byte strings into encoded line identifiers.
    #[must_use]
    pub fn diff_lines_to_chars<A: AsRef<[u8]>, B: AsRef<[u8]>>(
        &self,
        text1: A,
        text2: B,
    ) -> (Text, Text, Vec<Text>) {
        let (indexes1, indexes2, lines) =
            self.diff_lines_to_indexes(text1.as_ref(), text2.as_ref());
        (
            Text::from(indexes_to_string(&indexes1)),
            Text::from(indexes_to_string(&indexes2)),
            lines,
        )
    }

    /// Split two byte strings into rune-valued line identifiers.
    #[must_use]
    pub fn diff_lines_to_runes<A: AsRef<[u8]>, B: AsRef<[u8]>>(
        &self,
        text1: A,
        text2: B,
    ) -> (Vec<u32>, Vec<u32>, Vec<Text>) {
        let (indexes1, indexes2, lines) =
            self.diff_lines_to_indexes(text1.as_ref(), text2.as_ref());
        let runes1 = string_to_indexes(&indexes_to_string(&indexes1));
        let runes2 = string_to_indexes(&indexes_to_string(&indexes2));
        (runes1, runes2, lines)
    }

    fn diff_lines_to_indexes(&self, text1: &[u8], text2: &[u8]) -> (Vec<u32>, Vec<u32>, Vec<Text>) {
        let mut lines = vec![Text::new()];
        let mut line_hash: HashMap<Vec<u8>, usize> = HashMap::new();
        let indexes1 = self.diff_lines_munge(text1, &mut lines, &mut line_hash);
        let indexes2 = self.diff_lines_munge(text2, &mut lines, &mut line_hash);
        (indexes1, indexes2, lines)
    }

    fn diff_lines_munge(
        &self,
        bytes: &[u8],
        lines: &mut Vec<Text>,
        line_hash: &mut HashMap<Vec<u8>, usize>,
    ) -> Vec<u32> {
        let mut result = Vec::new();
        let mut start = 0;
        while start < bytes.len() {
            let end = bytes[start..]
                .iter()
                .position(|&byte| byte == b'\n')
                .map_or(bytes.len(), |relative| start + relative + 1);
            let line = &bytes[start..end];
            let value = if let Some(&existing) = line_hash.get(line) {
                existing
            } else {
                lines.push(Text::from(line));
                let value = lines.len() - 1;
                line_hash.insert(line.to_vec(), value);
                value
            };
            result.push(value as u32);
            start = end;
        }
        result
    }

    /// Rehydrate line identifiers inside diffs.
    #[must_use]
    pub fn diff_chars_to_lines(&self, diffs: &[Diff], lines: &[Text]) -> Vec<Diff> {
        diffs
            .iter()
            .map(|diff| {
                let mut hydrated = Vec::new();
                for rune in decode_go_runes(diff.text.as_bytes()) {
                    let index = rune_to_int(rune) as usize;
                    hydrated.extend_from_slice(lines[index].as_bytes());
                }
                Diff::new(diff.operation, hydrated)
            })
            .collect()
    }

    #[must_use]
    pub fn diff_common_prefix<A: AsRef<[u8]>, B: AsRef<[u8]>>(&self, text1: A, text2: B) -> usize {
        common_prefix_runes(
            &decode_go_runes(text1.as_ref()),
            &decode_go_runes(text2.as_ref()),
        )
    }

    #[must_use]
    pub fn diff_common_suffix<A: AsRef<[u8]>, B: AsRef<[u8]>>(&self, text1: A, text2: B) -> usize {
        common_suffix_runes(
            &decode_go_runes(text1.as_ref()),
            &decode_go_runes(text2.as_ref()),
        )
    }

    #[must_use]
    pub fn diff_common_overlap<A: AsRef<[u8]>, B: AsRef<[u8]>>(&self, text1: A, text2: B) -> usize {
        let mut first = text1.as_ref();
        let mut second = text2.as_ref();
        if first.is_empty() || second.is_empty() {
            return 0;
        }
        match first.len().cmp(&second.len()) {
            std::cmp::Ordering::Greater => {
                first = &first[first.len() - second.len()..];
            }
            std::cmp::Ordering::Less => {
                second = &second[..first.len()];
            }
            std::cmp::Ordering::Equal => {}
        }
        let text_length = first.len().min(second.len());
        if first == second {
            return text_length;
        }
        let mut best = 0;
        let mut length = 1;
        loop {
            let pattern = &first[text_length - length..];
            let Some(found) = find_subslice(second, pattern) else {
                break;
            };
            length += found;
            if found == 0 || first[text_length - length..] == second[..length] {
                best = length;
                length += 1;
                if length > text_length {
                    break;
                }
            }
        }
        best
    }

    #[must_use]
    pub fn diff_half_match<A: AsRef<[u8]>, B: AsRef<[u8]>>(
        &self,
        text1: A,
        text2: B,
    ) -> Option<[Text; 5]> {
        self.diff_half_match_runes(
            &decode_go_runes(text1.as_ref()),
            &decode_go_runes(text2.as_ref()),
        )
        .map(|parts| parts.map(|part| Text::from(encode_go_runes(&part))))
    }

    fn diff_half_match_runes(&self, text1: &[u32], text2: &[u32]) -> Option<[Vec<u32>; 5]> {
        if self.diff_timeout.is_zero() {
            return None;
        }
        let (long_text, short_text, first_is_long) = if text1.len() > text2.len() {
            (text1, text2, true)
        } else {
            (text2, text1, false)
        };
        if long_text.len() < 4 || short_text.len() * 2 < long_text.len() {
            return None;
        }
        let first = self.diff_half_match_at(long_text, short_text, (long_text.len() + 3) / 4);
        let second = self.diff_half_match_at(long_text, short_text, (long_text.len() + 1) / 2);
        let chosen = match (first, second) {
            (None, None) => return None,
            (Some(value), None) | (None, Some(value)) => value,
            (Some(left), Some(right)) => {
                if left[4].len() > right[4].len() {
                    left
                } else {
                    right
                }
            }
        };
        if first_is_long {
            Some(chosen)
        } else {
            Some([
                chosen[2].clone(),
                chosen[3].clone(),
                chosen[0].clone(),
                chosen[1].clone(),
                chosen[4].clone(),
            ])
        }
    }

    fn diff_half_match_at(
        &self,
        long_text: &[u32],
        short_text: &[u32],
        index: usize,
    ) -> Option<[Vec<u32>; 5]> {
        let seed = &long_text[index..index + long_text.len() / 4];
        let mut best_common_a = Vec::new();
        let mut best_common_b = Vec::new();
        let mut best_common_len = 0;
        let mut best_long_a = Vec::new();
        let mut best_long_b = Vec::new();
        let mut best_short_a = Vec::new();
        let mut best_short_b = Vec::new();
        let mut search = 0_isize;
        loop {
            let found = index_of_runes(short_text, seed, search);
            if found == -1 {
                break;
            }
            let found = found as usize;
            let prefix = common_prefix_runes(&long_text[index..], &short_text[found..]);
            let suffix = common_suffix_runes(&long_text[..index], &short_text[..found]);
            if best_common_len < suffix + prefix {
                best_common_a = short_text[found - suffix..found].to_vec();
                best_common_b = short_text[found..found + prefix].to_vec();
                best_common_len = suffix + prefix;
                best_long_a = long_text[..index - suffix].to_vec();
                best_long_b = long_text[index + prefix..].to_vec();
                best_short_a = short_text[..found - suffix].to_vec();
                best_short_b = short_text[found + prefix..].to_vec();
            }
            search = found as isize + 1;
        }
        if best_common_len * 2 < long_text.len() {
            return None;
        }
        best_common_a.extend(best_common_b);
        Some([
            best_long_a,
            best_long_b,
            best_short_a,
            best_short_b,
            best_common_a,
        ])
    }

    #[must_use]
    pub fn diff_cleanup_semantic(&self, mut diffs: Vec<Diff>) -> Vec<Diff> {
        let mut changes = false;
        let mut equalities = Vec::with_capacity(diffs.len());
        let mut last_equality = Vec::new();
        let mut pointer: isize = 0;
        let (mut insertions1, mut deletions1, mut insertions2, mut deletions2) = (0, 0, 0, 0);
        while pointer < diffs.len() as isize {
            let index = pointer as usize;
            if diffs[index].operation == DIFF_EQUAL {
                equalities.push(index);
                insertions1 = insertions2;
                deletions1 = deletions2;
                insertions2 = 0;
                deletions2 = 0;
                last_equality = diffs[index].text.as_bytes().to_vec();
            } else {
                if diffs[index].operation == DIFF_INSERT {
                    insertions2 += rune_count(diffs[index].text.as_bytes());
                } else {
                    deletions2 += rune_count(diffs[index].text.as_bytes());
                }
                let length = rune_count(&last_equality);
                if length > 0
                    && length <= insertions1.max(deletions1)
                    && length <= insertions2.max(deletions2)
                {
                    let insertion_point = *equalities.last().expect("equality index");
                    diffs.insert(
                        insertion_point,
                        Diff::new(DIFF_DELETE, last_equality.clone()),
                    );
                    diffs[insertion_point + 1].operation = DIFF_INSERT;
                    equalities.pop();
                    equalities.pop();
                    pointer = equalities.last().map_or(-1, |&value| value as isize);
                    insertions1 = 0;
                    deletions1 = 0;
                    insertions2 = 0;
                    deletions2 = 0;
                    last_equality.clear();
                    changes = true;
                }
            }
            pointer += 1;
        }
        if changes {
            diffs = self.diff_cleanup_merge(diffs);
        }
        diffs = self.diff_cleanup_semantic_lossless(diffs);
        pointer = 1;
        while pointer < diffs.len() as isize {
            let index = pointer as usize;
            if diffs[index - 1].operation == DIFF_DELETE && diffs[index].operation == DIFF_INSERT {
                let deletion = diffs[index - 1].text.as_bytes().to_vec();
                let insertion = diffs[index].text.as_bytes().to_vec();
                let overlap1 = self.diff_common_overlap(&deletion, &insertion);
                let overlap2 = self.diff_common_overlap(&insertion, &deletion);
                if overlap1 >= overlap2 {
                    if overlap1 as f64 >= rune_count(&deletion) as f64 / 2.0
                        || overlap1 as f64 >= rune_count(&insertion) as f64 / 2.0
                    {
                        diffs.insert(index, Diff::new(DIFF_EQUAL, insertion[..overlap1].to_vec()));
                        diffs[index - 1].text =
                            Text::from(deletion[..deletion.len() - overlap1].to_vec());
                        diffs[index + 1].text = Text::from(insertion[overlap1..].to_vec());
                        pointer += 1;
                    }
                } else if overlap2 as f64 >= rune_count(&deletion) as f64 / 2.0
                    || overlap2 as f64 >= rune_count(&insertion) as f64 / 2.0
                {
                    diffs.insert(index, Diff::new(DIFF_EQUAL, deletion[..overlap2].to_vec()));
                    diffs[index - 1].operation = DIFF_INSERT;
                    diffs[index - 1].text =
                        Text::from(insertion[..insertion.len() - overlap2].to_vec());
                    diffs[index + 1].operation = DIFF_DELETE;
                    diffs[index + 1].text = Text::from(deletion[overlap2..].to_vec());
                    pointer += 1;
                }
                pointer += 1;
            }
            pointer += 1;
        }
        diffs
    }

    #[must_use]
    pub fn diff_cleanup_semantic_lossless(&self, mut diffs: Vec<Diff>) -> Vec<Diff> {
        let mut pointer = 1_usize;
        while pointer + 1 < diffs.len() {
            if diffs[pointer - 1].operation == DIFF_EQUAL
                && diffs[pointer + 1].operation == DIFF_EQUAL
            {
                let mut equality1 = diffs[pointer - 1].text.as_bytes().to_vec();
                let mut edit = diffs[pointer].text.as_bytes().to_vec();
                let mut equality2 = diffs[pointer + 1].text.as_bytes().to_vec();
                let common_offset = self.diff_common_suffix(&equality1, &edit);
                if common_offset > 0 {
                    let common = edit[edit.len() - common_offset..].to_vec();
                    equality1.truncate(equality1.len() - common_offset);
                    let mut shifted = common.clone();
                    shifted.extend_from_slice(&edit[..edit.len() - common_offset]);
                    edit = shifted;
                    let mut shifted_equality = common;
                    shifted_equality.extend(equality2);
                    equality2 = shifted_equality;
                }
                let mut best_equality1 = equality1.clone();
                let mut best_edit = edit.clone();
                let mut best_equality2 = equality2.clone();
                let mut best_score =
                    semantic_score(&equality1, &edit) + semantic_score(&edit, &equality2);
                while !edit.is_empty() && !equality2.is_empty() {
                    let (_, width) = decode_go_rune(&edit);
                    if equality2.len() < width || edit[..width] != equality2[..width] {
                        break;
                    }
                    equality1.extend_from_slice(&edit[..width]);
                    let mut shifted_edit = edit[width..].to_vec();
                    shifted_edit.extend_from_slice(&equality2[..width]);
                    edit = shifted_edit;
                    equality2.drain(..width);
                    let score =
                        semantic_score(&equality1, &edit) + semantic_score(&edit, &equality2);
                    if score >= best_score {
                        best_score = score;
                        best_equality1 = equality1.clone();
                        best_edit = edit.clone();
                        best_equality2 = equality2.clone();
                    }
                }
                if diffs[pointer - 1].text.as_bytes() != best_equality1 {
                    if best_equality1.is_empty() {
                        diffs.remove(pointer - 1);
                        pointer -= 1;
                    } else {
                        diffs[pointer - 1].text = Text::from(best_equality1);
                    }
                    diffs[pointer].text = Text::from(best_edit);
                    if best_equality2.is_empty() {
                        diffs.remove(pointer + 1);
                        pointer = pointer.saturating_sub(1);
                    } else {
                        diffs[pointer + 1].text = Text::from(best_equality2);
                    }
                }
            }
            pointer += 1;
        }
        diffs
    }

    #[must_use]
    pub fn diff_cleanup_efficiency(&self, mut diffs: Vec<Diff>) -> Vec<Diff> {
        let mut changes = false;
        let mut equalities: Vec<usize> = Vec::new();
        let mut last_equality = Vec::new();
        let mut pointer: isize = 0;
        let (mut pre_insert, mut pre_delete, mut post_insert, mut post_delete) =
            (false, false, false, false);
        while pointer < diffs.len() as isize {
            let index = pointer as usize;
            if diffs[index].operation == DIFF_EQUAL {
                if diffs[index].text.len() < self.diff_edit_cost.max(0) as usize
                    && (post_insert || post_delete)
                {
                    equalities.push(index);
                    pre_insert = post_insert;
                    pre_delete = post_delete;
                    last_equality = diffs[index].text.as_bytes().to_vec();
                } else {
                    equalities.clear();
                    last_equality.clear();
                }
                post_insert = false;
                post_delete = false;
            } else {
                if diffs[index].operation == DIFF_DELETE {
                    post_delete = true;
                } else {
                    post_insert = true;
                }
                let sum = [pre_insert, pre_delete, post_insert, post_delete]
                    .into_iter()
                    .filter(|value| *value)
                    .count();
                if !last_equality.is_empty()
                    && ((pre_insert && pre_delete && post_insert && post_delete)
                        || (last_equality.len() < (self.diff_edit_cost / 2).max(0) as usize
                            && sum == 3))
                {
                    let insertion_point = *equalities.last().expect("equality index");
                    diffs.insert(
                        insertion_point,
                        Diff::new(DIFF_DELETE, last_equality.clone()),
                    );
                    diffs[insertion_point + 1].operation = DIFF_INSERT;
                    equalities.pop();
                    last_equality.clear();
                    if pre_insert && pre_delete {
                        post_insert = true;
                        post_delete = true;
                        equalities.clear();
                    } else {
                        equalities.pop();
                        pointer = equalities.last().map_or(-1, |&value| value as isize);
                        post_insert = false;
                        post_delete = false;
                    }
                    changes = true;
                }
            }
            pointer += 1;
        }
        if changes {
            self.diff_cleanup_merge(diffs)
        } else {
            diffs
        }
    }

    #[must_use]
    #[allow(clippy::only_used_in_recursion)]
    pub fn diff_cleanup_merge(&self, mut diffs: Vec<Diff>) -> Vec<Diff> {
        diffs.push(Diff::new(DIFF_EQUAL, b""));
        let mut pointer = 0_usize;
        let mut count_delete = 0_usize;
        let mut count_insert = 0_usize;
        let mut text_delete: Vec<u32> = Vec::new();
        let mut text_insert: Vec<u32> = Vec::new();
        while pointer < diffs.len() {
            match diffs[pointer].operation {
                DIFF_INSERT => {
                    count_insert += 1;
                    text_insert.extend(decode_go_runes(diffs[pointer].text.as_bytes()));
                    pointer += 1;
                }
                DIFF_DELETE => {
                    count_delete += 1;
                    text_delete.extend(decode_go_runes(diffs[pointer].text.as_bytes()));
                    pointer += 1;
                }
                DIFF_EQUAL => {
                    if count_delete + count_insert > 1 {
                        if count_delete != 0 && count_insert != 0 {
                            let prefix = common_prefix_runes(&text_insert, &text_delete);
                            if prefix != 0 {
                                let x = pointer - count_delete - count_insert;
                                let prefix_text = encode_go_runes(&text_insert[..prefix]);
                                if x > 0 && diffs[x - 1].operation == DIFF_EQUAL {
                                    diffs[x - 1].text.as_mut_bytes().extend(prefix_text);
                                } else {
                                    diffs.insert(0, Diff::new(DIFF_EQUAL, prefix_text));
                                    pointer += 1;
                                }
                                text_insert.drain(..prefix);
                                text_delete.drain(..prefix);
                            }
                            let suffix = common_suffix_runes(&text_insert, &text_delete);
                            if suffix != 0 {
                                let suffix_text =
                                    encode_go_runes(&text_insert[text_insert.len() - suffix..]);
                                let mut equality = suffix_text;
                                equality.extend_from_slice(diffs[pointer].text.as_bytes());
                                diffs[pointer].text = Text::from(equality);
                                text_insert.truncate(text_insert.len() - suffix);
                                text_delete.truncate(text_delete.len() - suffix);
                            }
                        }
                        let start = pointer - count_delete - count_insert;
                        let replacements = if count_delete == 0 {
                            vec![Diff::new(DIFF_INSERT, encode_go_runes(&text_insert))]
                        } else if count_insert == 0 {
                            vec![Diff::new(DIFF_DELETE, encode_go_runes(&text_delete))]
                        } else {
                            vec![
                                Diff::new(DIFF_DELETE, encode_go_runes(&text_delete)),
                                Diff::new(DIFF_INSERT, encode_go_runes(&text_insert)),
                            ]
                        };
                        diffs.splice(start..pointer, replacements);
                        pointer =
                            start + usize::from(count_delete != 0) + usize::from(count_insert != 0);
                    } else if pointer != 0 && diffs[pointer - 1].operation == DIFF_EQUAL {
                        let current = diffs[pointer].text.as_bytes().to_vec();
                        diffs[pointer - 1].text.as_mut_bytes().extend(current);
                        diffs.remove(pointer);
                    } else {
                        pointer += 1;
                    }
                    count_insert = 0;
                    count_delete = 0;
                    text_delete.clear();
                    text_insert.clear();
                }
                _ => pointer += 1,
            }
        }
        if diffs.last().is_some_and(|diff| diff.text.is_empty()) {
            diffs.pop();
        }

        let mut changes = false;
        pointer = 1;
        while pointer + 1 < diffs.len() {
            if diffs[pointer - 1].operation == DIFF_EQUAL
                && diffs[pointer + 1].operation == DIFF_EQUAL
            {
                let previous = diffs[pointer - 1].text.as_bytes().to_vec();
                let next = diffs[pointer + 1].text.as_bytes().to_vec();
                let edit = diffs[pointer].text.as_bytes().to_vec();
                if edit.ends_with(&previous) {
                    let mut shifted = previous.clone();
                    shifted.extend_from_slice(&edit[..edit.len() - previous.len()]);
                    diffs[pointer].text = Text::from(shifted);
                    let mut equality = previous;
                    equality.extend_from_slice(&next);
                    diffs[pointer + 1].text = Text::from(equality);
                    diffs.remove(pointer - 1);
                    changes = true;
                } else if edit.starts_with(&next) {
                    diffs[pointer - 1]
                        .text
                        .as_mut_bytes()
                        .extend_from_slice(&next);
                    let mut shifted = edit[next.len()..].to_vec();
                    shifted.extend_from_slice(&next);
                    diffs[pointer].text = Text::from(shifted);
                    diffs.remove(pointer + 1);
                    changes = true;
                }
            }
            pointer += 1;
        }
        if changes {
            self.diff_cleanup_merge(diffs)
        } else {
            diffs
        }
    }

    #[must_use]
    pub fn diff_x_index(&self, diffs: &[Diff], location: isize) -> isize {
        let (mut chars1, mut chars2, mut last1, mut last2) = (0_isize, 0_isize, 0_isize, 0_isize);
        let mut last_operation = Operation::EQUAL;
        for diff in diffs {
            if diff.operation != DIFF_INSERT {
                chars1 += diff.text.len() as isize;
            }
            if diff.operation != DIFF_DELETE {
                chars2 += diff.text.len() as isize;
            }
            if chars1 > location {
                last_operation = diff.operation;
                break;
            }
            last1 = chars1;
            last2 = chars2;
        }
        if last_operation == DIFF_DELETE {
            last2
        } else {
            last2 + location - last1
        }
    }

    #[must_use]
    pub fn diff_pretty_html(&self, diffs: &[Diff]) -> Text {
        let mut output = Vec::new();
        for diff in diffs {
            let mut escaped = html_escape(diff.text.as_bytes());
            escaped = replace_all(&escaped, b"\n", b"&para;<br>");
            match diff.operation {
                DIFF_INSERT => {
                    output.extend_from_slice(b"<ins style=\"background:#e6ffe6;\">");
                    output.extend(escaped);
                    output.extend_from_slice(b"</ins>");
                }
                DIFF_DELETE => {
                    output.extend_from_slice(b"<del style=\"background:#ffe6e6;\">");
                    output.extend(escaped);
                    output.extend_from_slice(b"</del>");
                }
                DIFF_EQUAL => {
                    output.extend_from_slice(b"<span>");
                    output.extend(escaped);
                    output.extend_from_slice(b"</span>");
                }
                _ => {}
            }
        }
        Text::from(output)
    }

    #[must_use]
    pub fn diff_pretty_text(&self, diffs: &[Diff]) -> Text {
        let mut output = Vec::new();
        for diff in diffs {
            match diff.operation {
                DIFF_INSERT | DIFF_DELETE => {
                    let color: &[u8] = if diff.operation == DIFF_INSERT {
                        b"\x1b[32m"
                    } else {
                        b"\x1b[31m"
                    };
                    let lines = split_bytes(diff.text.as_bytes(), b'\n');
                    for (index, line) in lines.iter().enumerate() {
                        output.extend_from_slice(color);
                        output.extend_from_slice(line);
                        if index + 1 < lines.len() {
                            output.extend_from_slice(b"\x1b[0m\n");
                        } else {
                            output.extend_from_slice(b"\x1b[0m");
                        }
                    }
                }
                DIFF_EQUAL => output.extend_from_slice(diff.text.as_bytes()),
                _ => {}
            }
        }
        Text::from(output)
    }

    #[must_use]
    pub fn diff_text1(&self, diffs: &[Diff]) -> Text {
        let mut output = Vec::new();
        for diff in diffs {
            if diff.operation != DIFF_INSERT {
                output.extend_from_slice(diff.text.as_bytes());
            }
        }
        Text::from(output)
    }

    #[must_use]
    pub fn diff_text2(&self, diffs: &[Diff]) -> Text {
        let mut output = Vec::new();
        for diff in diffs {
            if diff.operation != DIFF_DELETE {
                output.extend_from_slice(diff.text.as_bytes());
            }
        }
        Text::from(output)
    }

    #[must_use]
    pub fn diff_levenshtein(&self, diffs: &[Diff]) -> usize {
        let (mut distance, mut insertions, mut deletions) = (0, 0, 0);
        for diff in diffs {
            match diff.operation {
                DIFF_INSERT => insertions += rune_count(diff.text.as_bytes()),
                DIFF_DELETE => deletions += rune_count(diff.text.as_bytes()),
                DIFF_EQUAL => {
                    distance += insertions.max(deletions);
                    insertions = 0;
                    deletions = 0;
                }
                _ => {}
            }
        }
        distance + insertions.max(deletions)
    }

    #[must_use]
    pub fn diff_to_delta(&self, diffs: &[Diff]) -> Text {
        let mut output = Vec::new();
        for diff in diffs {
            match diff.operation {
                DIFF_INSERT => {
                    output.push(b'+');
                    output.extend(escaped_for_patch(diff.text.as_bytes()));
                    output.push(b'\t');
                }
                DIFF_DELETE => {
                    output.extend_from_slice(
                        format!("-{}\t", rune_count(diff.text.as_bytes())).as_bytes(),
                    );
                }
                DIFF_EQUAL => {
                    output.extend_from_slice(
                        format!("={}\t", rune_count(diff.text.as_bytes())).as_bytes(),
                    );
                }
                _ => {}
            }
        }
        output.pop();
        Text::from(output)
    }

    pub fn diff_from_delta<A: AsRef<[u8]>, B: AsRef<[u8]>>(
        &self,
        text1: A,
        delta: B,
    ) -> Result<Vec<Diff>, PatchError> {
        let source = text1.as_ref();
        let source_runes = decode_go_runes(source);
        let mut consumed = 0_usize;
        let mut diffs = Vec::new();
        for token in split_bytes(delta.as_ref(), b'\t') {
            if token.is_empty() {
                continue;
            }
            let operation = token[0];
            let parameter = &token[1..];
            match operation {
                b'+' => {
                    let encoded = replace_all(parameter, b"+", b"%2b");
                    let decoded = query_unescape(&encoded)?;
                    if std::str::from_utf8(&decoded).is_err() {
                        return Err(PatchError::InvalidUtf8Token(Text::from(decoded)));
                    }
                    diffs.push(Diff::new(DIFF_INSERT, decoded));
                }
                b'=' | b'-' => {
                    let number_text = String::from_utf8_lossy(parameter).into_owned();
                    let number = number_text.parse::<isize>().map_err(|_| {
                        PatchError::InvalidNumber(format!(
                            "strconv.ParseInt: parsing \"{number_text}\": invalid syntax"
                        ))
                    })?;
                    if number < 0 {
                        return Err(PatchError::NegativeNumber(number_text));
                    }
                    consumed += number as usize;
                    if consumed > source_runes.len() {
                        break;
                    }
                    let fragment =
                        encode_go_runes(&source_runes[consumed - number as usize..consumed]);
                    diffs.push(Diff::new(
                        if operation == b'=' {
                            DIFF_EQUAL
                        } else {
                            DIFF_DELETE
                        },
                        fragment,
                    ));
                }
                other => return Err(PatchError::InvalidDiffOperation(other)),
            }
        }
        if consumed != source_runes.len() {
            return Err(PatchError::DeltaLength {
                consumed,
                source_bytes: source.len(),
            });
        }
        Ok(diffs)
    }
}

fn semantic_score(left: &[u8], right: &[u8]) -> usize {
    if left.is_empty() || right.is_empty() {
        return 6;
    }
    let left_runes = decode_go_runes(left);
    let right_runes = decode_go_runes(right);
    let left_rune = *left_runes.last().unwrap_or(&0);
    let right_rune = right_runes.first().copied().unwrap_or(0);
    let non_alnum_left = !is_ascii_alphanumeric(left_rune);
    let non_alnum_right = !is_ascii_alphanumeric(right_rune);
    let whitespace_left = non_alnum_left && is_go_whitespace(left_rune);
    let whitespace_right = non_alnum_right && is_go_whitespace(right_rune);
    let line_break_left = whitespace_left && matches!(left_rune, 10 | 13);
    let line_break_right = whitespace_right && matches!(right_rune, 10 | 13);
    let blank_line_left = line_break_left && blank_line_end(left);
    // go-diff v1.4.0 applies its end-of-blank-line expression to both sides.
    // Although it also declares a start expression, that expression is unused.
    // Preserve the observable upstream behavior for strict compatibility.
    let blank_line_right = line_break_right && blank_line_end(right);
    if blank_line_left || blank_line_right {
        5
    } else if line_break_left || line_break_right {
        4
    } else if non_alnum_left && !whitespace_left && whitespace_right {
        3
    } else if whitespace_left || whitespace_right {
        2
    } else if non_alnum_left || non_alnum_right {
        1
    } else {
        0
    }
}

fn is_ascii_alphanumeric(rune: u32) -> bool {
    matches!(rune, 0x30..=0x39 | 0x41..=0x5A | 0x61..=0x7A)
}

fn is_go_whitespace(rune: u32) -> bool {
    matches!(rune, 9 | 10 | 12 | 13 | 32)
}

fn blank_line_end(bytes: &[u8]) -> bool {
    bytes.ends_with(b"\n\n") || bytes.ends_with(b"\n\r\n")
}

fn replace_all(bytes: &[u8], pattern: &[u8], replacement: &[u8]) -> Vec<u8> {
    if pattern.is_empty() {
        return bytes.to_vec();
    }
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(pattern) {
            output.extend_from_slice(replacement);
            index += pattern.len();
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_utf8_is_replaced_like_go() {
        let dmp = DiffMatchPatch::new();
        assert_eq!(
            dmp.diff_main(b"\xe0\xe5", b"", false),
            vec![Diff::new(DIFF_DELETE, "��")]
        );
    }
}
