//! Question block recognition and MC, MA, NUM, and FIB parsing.

use qti_core::{Item, ItemBody};

pub(super) fn split_questions(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current = Vec::new();
    let mut previous_blank = true;
    for raw_line in text.trim().lines() {
        let line = raw_line.trim_end();
        if question_start(line) && previous_blank && !current.is_empty() {
            blocks.push(current.join("\n").trim().to_owned());
            current.clear();
        }
        current.push(line);
        previous_blank = line.is_empty();
    }
    if !current.is_empty() {
        blocks.push(current.join("\n").trim().to_owned());
    }
    blocks
}

fn question_start(line: &str) -> bool {
    let bytes = line.as_bytes();
    let mut position = 0;
    while bytes.get(position).is_some_and(u8::is_ascii_digit) {
        position += 1;
    }
    position > 0 && bytes.get(position) == Some(&b'.') && bytes.get(position + 1) == Some(&b' ')
}

pub(super) fn parse_block(block: &str) -> Result<Option<Item>, String> {
    let lines = block.lines().map(str::trim_end).collect::<Vec<_>>();
    let Some(number_end) = question_number_end(lines.first().copied().unwrap_or_default()) else {
        return Ok(None);
    };
    if count_ma(&lines) >= 3 {
        parse_ma(&lines, number_end).map(Some)
    } else if count_mc(&lines) >= 2 {
        parse_mc(&lines, number_end).map(Some)
    } else if lines.iter().any(|line| line.starts_with('=')) {
        parse_num(&lines, number_end).map(Some)
    } else if lines.iter().any(|line| line.starts_with("* ")) {
        parse_fib(&lines, number_end).map(Some)
    } else {
        Ok(None)
    }
}

fn question_number_end(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut position = 0;
    while bytes.get(position).is_some_and(u8::is_ascii_digit) {
        position += 1;
    }
    (position > 0 && bytes.get(position) == Some(&b'.') && bytes.get(position + 1) == Some(&b' '))
        .then_some(position + 2)
}

fn count_mc(lines: &[&str]) -> usize {
    lines
        .iter()
        .filter(|line| mc_choice(line).is_some())
        .count()
}

fn count_ma(lines: &[&str]) -> usize {
    lines
        .iter()
        .filter(|line| ma_choice(line).is_some())
        .count()
}

fn mc_choice(line: &str) -> Option<(bool, String)> {
    let trimmed = line.trim_start();
    let (correct, rest) = trimmed
        .strip_prefix('*')
        .map_or((false, trimmed), |rest| (true, rest));
    let bytes = rest.as_bytes();
    (bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b')'
        && bytes[2].is_ascii_whitespace())
    .then(|| (correct, rest[3..].trim().to_owned()))
}

fn ma_choice(line: &str) -> Option<(bool, String)> {
    let trimmed = line.trim();
    let content = trimmed
        .strip_prefix("[*] ")
        .map(|text| (true, text))
        .or_else(|| trimmed.strip_prefix("[ ] ").map(|text| (false, text)))?;
    Some((content.0, content.1.trim().to_owned()))
}

fn stem(lines: &[&str], end: usize, answer_start: usize) -> String {
    let mut pieces = Vec::new();
    if let Some(first) = lines.first() {
        pieces.push(first[end..].trim());
    }
    pieces.extend(
        lines
            .iter()
            .take(answer_start)
            .skip(1)
            .map(|line| line.trim()),
    );
    pieces
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_mc(lines: &[&str], number_end: usize) -> Result<Item, String> {
    let start = lines
        .iter()
        .position(|line| mc_choice(line).is_some())
        .ok_or_else(|| "MC has no choices".to_owned())?;
    let question = stem(lines, number_end, start);
    let mut choices = Vec::new();
    let mut answer = None;
    let mut current = None::<(bool, String)>;
    for line in &lines[start..] {
        if let Some(next) = mc_choice(line) {
            if let Some((correct, choice)) = current.take() {
                if correct {
                    if answer.is_some() {
                        return Err("MC has more than one correct choice".to_owned());
                    }
                    answer = Some(choice.clone());
                }
                choices.push(choice);
            }
            current = Some(next);
        } else if !feedback_line(line)
            && let Some((_, choice)) = &mut current
        {
            append_line(choice, line);
        }
    }
    if let Some((correct, choice)) = current {
        if correct {
            if answer.is_some() {
                return Err("MC has more than one correct choice".to_owned());
            }
            answer = Some(choice.clone());
        }
        choices.push(choice);
    }
    Item::new(
        question,
        ItemBody::Mc {
            choices,
            answer: answer.ok_or_else(|| "MC has no correct choice".to_owned())?,
        },
    )
    .map_err(|error| error.to_string())
}

fn parse_ma(lines: &[&str], number_end: usize) -> Result<Item, String> {
    let start = lines
        .iter()
        .position(|line| ma_choice(line).is_some())
        .ok_or_else(|| "MA has no choices".to_owned())?;
    let question = stem(lines, number_end, start);
    let mut choices = Vec::new();
    let mut answers = Vec::new();
    let mut current = None::<(bool, String)>;
    for line in &lines[start..] {
        if let Some(next) = ma_choice(line) {
            if let Some((correct, choice)) = current.take() {
                if correct {
                    answers.push(choice.clone());
                }
                choices.push(choice);
            }
            current = Some(next);
        } else if !feedback_line(line)
            && let Some((_, choice)) = &mut current
        {
            append_line(choice, line);
        }
    }
    if let Some((correct, choice)) = current {
        if correct {
            answers.push(choice.clone());
        }
        choices.push(choice);
    }
    Item::new(
        question,
        ItemBody::Ma {
            choices,
            answers,
            // text2qti carries choices and answer markers but no MA grading options.
            // The frozen reader delegates omitted fields to the MA constructor defaults.
            min_answers_required: 1,
            allow_all_correct: true,
        },
    )
    .map_err(|error| error.to_string())
}

fn parse_num(lines: &[&str], number_end: usize) -> Result<Item, String> {
    let start = lines
        .iter()
        .position(|line| line.starts_with('='))
        .ok_or_else(|| "NUM lacks answer line".to_owned())?;
    let question = stem(lines, number_end, start);
    let value = lines[start].trim_start_matches('=').trim();
    let (answer, tolerance) = if let Some(range) = value
        .strip_prefix('[')
        .and_then(|part| part.strip_suffix(']'))
    {
        let (low, high) = range
            .split_once(',')
            .ok_or_else(|| "invalid NUM range".to_owned())?;
        let low = parse_number(low)?;
        let high = parse_number(high)?;
        ((low + high) / 2.0, (high - low) / 2.0)
    } else if let Some((answer, tolerance)) = value.split_once("+-") {
        (parse_number(answer)?, parse_number(tolerance)?)
    } else {
        (parse_number(value)?, 0.0)
    };
    Item::new(
        question,
        ItemBody::Num {
            answer,
            tolerance,
            tolerance_message: true,
        },
    )
    .map_err(|error| error.to_string())
}

fn parse_number(value: &str) -> Result<f64, String> {
    let number = value
        .trim()
        .replace('_', "")
        .parse::<f64>()
        .map_err(|_| "invalid NUM answer".to_owned())?;
    number
        .is_finite()
        .then_some(number)
        .ok_or_else(|| "NUM values must be finite".to_owned())
}

fn parse_fib(lines: &[&str], number_end: usize) -> Result<Item, String> {
    let start = lines
        .iter()
        .position(|line| line.starts_with("* "))
        .ok_or_else(|| "FIB has no answers".to_owned())?;
    let question = stem(lines, number_end, start);
    let answers = lines[start..]
        .iter()
        .filter_map(|line| {
            line.strip_prefix("* ")
                .map(|answer| answer.trim().to_owned())
        })
        .collect();
    Item::new(question, ItemBody::Fib { answers }).map_err(|error| error.to_string())
}

fn feedback_line(line: &str) -> bool {
    ["... ", "+ ", "- "]
        .iter()
        .any(|prefix| line.starts_with(prefix))
}

fn append_line(target: &mut String, line: &str) {
    let text = line.trim();
    if !text.is_empty() {
        if !target.is_empty() {
            target.push(' ');
        }
        target.push_str(text);
    }
}
