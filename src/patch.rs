// Direct behavioral port of diffmatchpatch/patch.go from sergi/go-diff v1.4.0.

use crate::util::{escaped_for_patch, find_subslice, query_unescape, split_bytes};
use crate::{DIFF_DELETE, DIFF_EQUAL, DIFF_INSERT, Diff, DiffMatchPatch, Patch, PatchError, Text};
use std::fmt;

impl Patch {
    #[must_use]
    pub fn to_text(&self) -> Text {
        let coordinates1 = coordinates(self.start1, self.length1);
        let coordinates2 = coordinates(self.start2, self.length2);
        let mut output = format!("@@ -{coordinates1} +{coordinates2} @@\n").into_bytes();
        for diff in &self.diffs {
            output.push(match diff.operation {
                DIFF_INSERT => b'+',
                DIFF_DELETE => b'-',
                _ => b' ',
            });
            output.extend(escaped_for_patch(diff.text.as_bytes()));
            output.push(b'\n');
        }
        Text::from(output)
    }
}

impl fmt::Display for Patch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_text().to_string_lossy())
    }
}

fn coordinates(start: isize, length: isize) -> String {
    if length == 0 {
        format!("{start},0")
    } else if length == 1 {
        (start + 1).to_string()
    } else {
        format!("{},{}", start + 1, length)
    }
}

impl DiffMatchPatch {
    #[must_use]
    pub fn patch_add_context<A: AsRef<[u8]>>(&self, mut patch: Patch, text: A) -> Patch {
        let text = text.as_ref();
        if text.is_empty() {
            return patch;
        }
        let start2 = patch.start2 as usize;
        let mut pattern = text[start2..start2 + patch.length1 as usize].to_vec();
        let mut padding = 0_isize;
        while find_subslice(text, &pattern) != crate::util::rfind_subslice(text, &pattern)
            && (pattern.len() as isize) < self.match_max_bits - 2 * self.patch_margin
        {
            padding += self.patch_margin;
            let start = (patch.start2 - padding).max(0) as usize;
            let end = (patch.start2 + patch.length1 + padding).min(text.len() as isize) as usize;
            pattern = text[start..end].to_vec();
        }
        padding += self.patch_margin;
        let prefix_start = (patch.start2 - padding).max(0) as usize;
        let prefix = text[prefix_start..start2].to_vec();
        if !prefix.is_empty() {
            patch.diffs.insert(0, Diff::new(DIFF_EQUAL, prefix.clone()));
        }
        let suffix_start = start2 + patch.length1 as usize;
        let suffix_end = (patch.start2 + patch.length1 + padding).min(text.len() as isize) as usize;
        let suffix = text[suffix_start..suffix_end].to_vec();
        if !suffix.is_empty() {
            patch.diffs.push(Diff::new(DIFF_EQUAL, suffix.clone()));
        }
        patch.start1 -= prefix.len() as isize;
        patch.start2 -= prefix.len() as isize;
        patch.length1 += (prefix.len() + suffix.len()) as isize;
        patch.length2 += (prefix.len() + suffix.len()) as isize;
        patch
    }

    /// Construct patches from source and destination text.
    #[must_use]
    pub fn patch_make<A: AsRef<[u8]>, B: AsRef<[u8]>>(&self, text1: A, text2: B) -> Vec<Patch> {
        let text1 = text1.as_ref();
        let mut diffs = self.diff_main(text1, text2.as_ref(), true);
        if diffs.len() > 2 {
            diffs = self.diff_cleanup_semantic(diffs);
            diffs = self.diff_cleanup_efficiency(diffs);
        }
        self.patch_make_from_text_and_diffs(text1, &diffs)
    }

    /// Construct patches from a diff sequence.
    #[must_use]
    pub fn patch_make_from_diffs(&self, diffs: &[Diff]) -> Vec<Patch> {
        let text1 = self.diff_text1(diffs);
        self.patch_make_from_text_and_diffs(text1.as_bytes(), diffs)
    }

    /// Construct patches from source text and a precomputed diff sequence.
    #[must_use]
    pub fn patch_make_from_text_and_diffs<A: AsRef<[u8]>>(
        &self,
        text1: A,
        diffs: &[Diff],
    ) -> Vec<Patch> {
        let mut patches = Vec::new();
        if diffs.is_empty() {
            return patches;
        }
        let mut patch = Patch::default();
        let (mut char_count1, mut char_count2) = (0_isize, 0_isize);
        let mut prepatch_text = text1.as_ref().to_vec();
        let mut postpatch_text = prepatch_text.clone();
        for (index, diff) in diffs.iter().enumerate() {
            if patch.diffs.is_empty() && diff.operation != DIFF_EQUAL {
                patch.start1 = char_count1;
                patch.start2 = char_count2;
            }
            match diff.operation {
                DIFF_INSERT => {
                    patch.diffs.push(diff.clone());
                    patch.length2 += diff.text.len() as isize;
                    postpatch_text.splice(
                        char_count2 as usize..char_count2 as usize,
                        diff.text.as_bytes().iter().copied(),
                    );
                }
                DIFF_DELETE => {
                    patch.length1 += diff.text.len() as isize;
                    patch.diffs.push(diff.clone());
                    let start = char_count2 as usize;
                    postpatch_text.drain(start..start + diff.text.len());
                }
                DIFF_EQUAL => {
                    if diff.text.len() <= (2 * self.patch_margin).max(0) as usize
                        && !patch.diffs.is_empty()
                        && index + 1 != diffs.len()
                    {
                        patch.diffs.push(diff.clone());
                        patch.length1 += diff.text.len() as isize;
                        patch.length2 += diff.text.len() as isize;
                    }
                    if diff.text.len() >= (2 * self.patch_margin).max(0) as usize
                        && !patch.diffs.is_empty()
                    {
                        patch = self.patch_add_context(patch, &prepatch_text);
                        patches.push(patch);
                        patch = Patch::default();
                        prepatch_text.clone_from(&postpatch_text);
                        char_count1 = char_count2;
                    }
                }
                _ => {}
            }
            if diff.operation != DIFF_INSERT {
                char_count1 += diff.text.len() as isize;
            }
            if diff.operation != DIFF_DELETE {
                char_count2 += diff.text.len() as isize;
            }
        }
        if !patch.diffs.is_empty() {
            patches.push(self.patch_add_context(patch, &prepatch_text));
        }
        patches
    }

    /// Deprecated Go three-argument form. The second text is intentionally ignored.
    #[must_use]
    pub fn patch_make_deprecated<A: AsRef<[u8]>, B: AsRef<[u8]>>(
        &self,
        text1: A,
        _text2: B,
        diffs: &[Diff],
    ) -> Vec<Patch> {
        self.patch_make_from_text_and_diffs(text1, diffs)
    }

    #[must_use]
    pub fn patch_deep_copy(&self, patches: &[Patch]) -> Vec<Patch> {
        patches.to_vec()
    }

    /// Apply patches without mutating the caller's patch list.
    #[must_use]
    pub fn patch_apply<A: AsRef<[u8]>>(&self, patches: &[Patch], text: A) -> (Text, Vec<bool>) {
        if patches.is_empty() {
            return (Text::from(text.as_ref()), Vec::new());
        }
        let mut patches = self.patch_deep_copy(patches);
        let null_padding = self.patch_add_padding(&mut patches);
        let mut working = null_padding.as_bytes().to_vec();
        working.extend_from_slice(text.as_ref());
        working.extend_from_slice(null_padding.as_bytes());
        patches = self.patch_split_max(patches);
        let mut delta = 0_isize;
        let mut results = vec![false; patches.len()];
        for (index, patch) in patches.iter().enumerate() {
            let expected_location = patch.start2 + delta;
            let text1 = self.diff_text1(&patch.diffs);
            let mut end_location = -1_isize;
            let start_location = if text1.len() as isize > self.match_max_bits {
                let bits = self.match_max_bits.max(0) as usize;
                let start = self.match_main(&working, &text1.as_bytes()[..bits], expected_location);
                if start != -1 {
                    end_location = self.match_main(
                        &working,
                        &text1.as_bytes()[text1.len() - bits..],
                        expected_location + text1.len() as isize - self.match_max_bits,
                    );
                    if end_location == -1 || start >= end_location {
                        -1
                    } else {
                        start
                    }
                } else {
                    -1
                }
            } else {
                self.match_main(&working, &text1, expected_location)
            };

            if start_location == -1 {
                results[index] = false;
                delta -= patch.length2 - patch.length1;
                continue;
            }
            results[index] = true;
            delta = start_location - expected_location;
            let start = start_location as usize;
            let text2 = if end_location == -1 {
                working[start..(start + text1.len()).min(working.len())].to_vec()
            } else {
                let end = (end_location + self.match_max_bits).min(working.len() as isize) as usize;
                working[start..end].to_vec()
            };
            if text1.as_bytes() == text2 {
                let replacement = self.diff_text2(&patch.diffs);
                working.splice(
                    start..start + text1.len(),
                    replacement.as_bytes().iter().copied(),
                );
                continue;
            }
            let mut framework = self.diff_main(&text1, &text2, false);
            if text1.len() as isize > self.match_max_bits
                && self.diff_levenshtein(&framework) as f64 / text1.len() as f64
                    > self.patch_delete_threshold
            {
                results[index] = false;
                continue;
            }
            framework = self.diff_cleanup_semantic_lossless(framework);
            let mut index1 = 0_isize;
            for diff in &patch.diffs {
                if diff.operation != DIFF_EQUAL {
                    let index2 = self.diff_x_index(&framework, index1);
                    if diff.operation == DIFF_INSERT {
                        let insertion = (start_location + index2) as usize;
                        working.splice(insertion..insertion, diff.text.as_bytes().iter().copied());
                    } else if diff.operation == DIFF_DELETE {
                        let deletion_start = (start_location + index2) as usize;
                        let deletion_end = (start_location
                            + self.diff_x_index(&framework, index1 + diff.text.len() as isize))
                            as usize;
                        working.drain(deletion_start..deletion_end);
                    }
                }
                if diff.operation != DIFF_DELETE {
                    index1 += diff.text.len() as isize;
                }
            }
        }
        let padding = null_padding.len();
        let end = padding + working.len() - 2 * padding;
        (Text::from(working[padding..end].to_vec()), results)
    }

    /// Mutate patches to include edge padding and return the padding bytes.
    #[must_use]
    pub fn patch_add_padding(&self, patches: &mut [Patch]) -> Text {
        let padding_length = self.patch_margin.max(0) as usize;
        let null_padding: Vec<u8> = (1..=padding_length).map(|value| value as u8).collect();
        for patch in patches.iter_mut() {
            patch.start1 += padding_length as isize;
            patch.start2 += padding_length as isize;
        }
        let first = &mut patches[0];
        if first
            .diffs
            .first()
            .is_none_or(|diff| diff.operation != DIFF_EQUAL)
        {
            first
                .diffs
                .insert(0, Diff::new(DIFF_EQUAL, null_padding.clone()));
            first.start1 -= padding_length as isize;
            first.start2 -= padding_length as isize;
            first.length1 += padding_length as isize;
            first.length2 += padding_length as isize;
        } else if padding_length > first.diffs[0].text.len() {
            let existing = first.diffs[0].text.len();
            let extra = padding_length - existing;
            let mut grown = null_padding[existing..].to_vec();
            grown.extend_from_slice(first.diffs[0].text.as_bytes());
            first.diffs[0].text = Text::from(grown);
            first.start1 -= extra as isize;
            first.start2 -= extra as isize;
            first.length1 += extra as isize;
            first.length2 += extra as isize;
        }
        let last_index = patches.len() - 1;
        let last = &mut patches[last_index];
        if last
            .diffs
            .last()
            .is_none_or(|diff| diff.operation != DIFF_EQUAL)
        {
            last.diffs.push(Diff::new(DIFF_EQUAL, null_padding.clone()));
            last.length1 += padding_length as isize;
            last.length2 += padding_length as isize;
        } else {
            let final_index = last.diffs.len() - 1;
            let existing = last.diffs[final_index].text.len();
            if padding_length > existing {
                let extra = padding_length - existing;
                last.diffs[final_index]
                    .text
                    .as_mut_bytes()
                    .extend_from_slice(&null_padding[..extra]);
                last.length1 += extra as isize;
                last.length2 += extra as isize;
            }
        }
        Text::from(null_padding)
    }

    #[must_use]
    pub fn patch_split_max(&self, mut patches: Vec<Patch>) -> Vec<Patch> {
        let patch_size = self.match_max_bits.max(0) as usize;
        let margin = self.patch_margin.max(0) as usize;
        let mut index = 0_usize;
        while index < patches.len() {
            if patches[index].length1 <= patch_size as isize {
                index += 1;
                continue;
            }
            let mut big_patch = patches.remove(index);
            let (mut start1, mut start2) = (big_patch.start1, big_patch.start2);
            let mut pre_context = Vec::new();
            while !big_patch.diffs.is_empty() {
                let mut patch = Patch {
                    start1: start1 - pre_context.len() as isize,
                    start2: start2 - pre_context.len() as isize,
                    ..Patch::default()
                };
                let mut empty = true;
                if !pre_context.is_empty() {
                    patch.length1 = pre_context.len() as isize;
                    patch.length2 = pre_context.len() as isize;
                    patch.diffs.push(Diff::new(DIFF_EQUAL, pre_context.clone()));
                }
                while !big_patch.diffs.is_empty()
                    && patch.length1 < patch_size.saturating_sub(margin) as isize
                {
                    let operation = big_patch.diffs[0].operation;
                    let mut diff_text = big_patch.diffs[0].text.as_bytes().to_vec();
                    if operation == DIFF_INSERT {
                        patch.length2 += diff_text.len() as isize;
                        start2 += diff_text.len() as isize;
                        patch.diffs.push(big_patch.diffs.remove(0));
                        empty = false;
                    } else if operation == DIFF_DELETE
                        && patch.diffs.len() == 1
                        && patch.diffs[0].operation == DIFF_EQUAL
                        && diff_text.len() > 2 * patch_size
                    {
                        patch.length1 += diff_text.len() as isize;
                        start1 += diff_text.len() as isize;
                        patch.diffs.push(big_patch.diffs.remove(0));
                        empty = false;
                    } else {
                        let take = diff_text
                            .len()
                            .min(patch_size.saturating_sub(patch.length1 as usize + margin));
                        diff_text.truncate(take);
                        patch.length1 += take as isize;
                        start1 += take as isize;
                        if operation == DIFF_EQUAL {
                            patch.length2 += take as isize;
                            start2 += take as isize;
                        } else {
                            empty = false;
                        }
                        patch.diffs.push(Diff::new(operation, diff_text.clone()));
                        if take == big_patch.diffs[0].text.len() {
                            big_patch.diffs.remove(0);
                        } else {
                            big_patch.diffs[0].text.as_mut_bytes().drain(..take);
                        }
                    }
                }
                pre_context = self.diff_text2(&patch.diffs).into_bytes();
                if pre_context.len() > margin {
                    pre_context = pre_context[pre_context.len() - margin..].to_vec();
                }
                let remaining = self.diff_text1(&big_patch.diffs);
                let post_context = remaining.as_bytes()[..remaining.len().min(margin)].to_vec();
                if !post_context.is_empty() {
                    patch.length1 += post_context.len() as isize;
                    patch.length2 += post_context.len() as isize;
                    match patch.diffs.last_mut() {
                        Some(last) if last.operation == DIFF_EQUAL => {
                            last.text.as_mut_bytes().extend_from_slice(&post_context);
                        }
                        _ => patch.diffs.push(Diff::new(DIFF_EQUAL, post_context)),
                    }
                }
                if !empty {
                    patches.insert(index, patch);
                    index += 1;
                }
            }
        }
        patches
    }

    #[must_use]
    pub fn patch_to_text(&self, patches: &[Patch]) -> Text {
        let mut output = Vec::new();
        for patch in patches {
            output.extend_from_slice(patch.to_text().as_bytes());
        }
        Text::from(output)
    }

    pub fn patch_from_text<A: AsRef<[u8]>>(&self, patch_text: A) -> Result<Vec<Patch>, PatchError> {
        let patch_text = patch_text.as_ref();
        if patch_text.is_empty() {
            return Ok(Vec::new());
        }
        let lines = split_bytes(patch_text, b'\n');
        let mut pointer = 0_usize;
        let mut patches = Vec::new();
        while pointer < lines.len() {
            let Some((start1, length1, start2, length2)) = parse_header(lines[pointer]) else {
                return Err(PatchError::InvalidPatchString(Text::from(lines[pointer])));
            };
            let mut patch = Patch {
                start1,
                start2,
                length1,
                length2,
                ..Patch::default()
            };
            pointer += 1;
            while pointer < lines.len() {
                if lines[pointer].is_empty() {
                    pointer += 1;
                    continue;
                }
                let mode = lines[pointer][0];
                if mode == b'@' {
                    break;
                }
                let encoded = replace_plus(&lines[pointer][1..]);
                let decoded = query_unescape(&encoded).unwrap_or_default();
                let operation = match mode {
                    b'-' => DIFF_DELETE,
                    b'+' => DIFF_INSERT,
                    b' ' => DIFF_EQUAL,
                    other => {
                        return Err(PatchError::InvalidPatchMode {
                            mode: other,
                            line: Text::from(decoded),
                        });
                    }
                };
                patch.diffs.push(Diff::new(operation, decoded));
                pointer += 1;
            }
            patches.push(patch);
        }
        Ok(patches)
    }
}

fn replace_plus(bytes: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(bytes.len());
    for &byte in bytes {
        if byte == b'+' {
            output.extend_from_slice(b"%2b");
        } else {
            output.push(byte);
        }
    }
    output
}

fn parse_header(line: &[u8]) -> Option<(isize, isize, isize, isize)> {
    if !line.starts_with(b"@@ -") || !line.ends_with(b" @@") {
        return None;
    }
    let body = &line[4..line.len() - 3];
    let separator = body.windows(2).position(|window| window == b" +")?;
    let left = &body[..separator];
    let right = &body[separator + 2..];
    let (start1, length1) = parse_coordinates(left)?;
    let (start2, length2) = parse_coordinates(right)?;
    Some((start1, length1, start2, length2))
}

fn parse_coordinates(bytes: &[u8]) -> Option<(isize, isize)> {
    let comma = bytes.iter().position(|&byte| byte == b',');
    let start_bytes = comma.map_or(bytes, |index| &bytes[..index]);
    if start_bytes.is_empty() || !start_bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let mut start = String::from_utf8_lossy(start_bytes).parse::<isize>().ok()?;
    let length = if let Some(comma) = comma {
        let length_bytes = &bytes[comma + 1..];
        if !length_bytes.iter().all(u8::is_ascii_digit) {
            return None;
        }
        if length_bytes.is_empty() {
            start -= 1;
            1
        } else {
            let parsed = String::from_utf8_lossy(length_bytes)
                .parse::<isize>()
                .ok()?;
            if parsed != 0 {
                start -= 1;
            }
            parsed
        }
    } else {
        start -= 1;
        1
    };
    Some((start, length))
}
