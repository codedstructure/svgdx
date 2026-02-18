use super::SvgElement;

pub fn get_md_value(element: &mut SvgElement) -> Vec<MdSpan> {
    let text_value = if let Some(tv) = element.pop_attr("md") {
        tv
    } else if let Some(tv) = element.pop_attr("text") {
        tv
    } else {
        return vec![];
    };

    // parse into spans and data about style
    let (spans, span_data) = md_parse(&text_value);

    let mut md_spans: Vec<MdSpan> = spans
        .iter()
        .map(|s| MdSpan {
            code: false,
            bold: false,
            italic: false,
            text: s.to_string(),
        })
        .collect();

    for s in span_data {
        let class = s.code_bold_italic;
        for i in md_spans.iter_mut().take(s.end_idx).skip(s.start_idx) {
            match class {
                SpanStyleEnum::Code => i.code = true,
                SpanStyleEnum::Bold => i.bold = true,
                SpanStyleEnum::Italic => i.italic = true,
            }
        }
    }

    // merge equal style spans together
    let mut result = vec![];
    let mut md_span_iter = md_spans.iter();
    if let Some(first) = md_span_iter.next() {
        result.push(first.clone());
    }
    for span in md_span_iter {
        if result[result.len() - 1].bold != span.bold
            || result[result.len() - 1].code != span.code
            || result[result.len() - 1].italic != span.italic
        {
            result.push(MdSpan {
                code: span.code,
                bold: span.bold,
                italic: span.italic,
                text: String::new(),
            });
        }
        let last_ind = result.len() - 1;
        result[last_ind].text += &span.text;
    }

    result
}

#[derive(Debug, Clone, PartialEq)]
pub struct MdSpan {
    pub code: bool,
    pub bold: bool,
    pub italic: bool,
    pub text: String,
}

#[derive(Debug, PartialEq)]
enum SpanStyleEnum {
    Code,
    Bold,
    Italic,
}

#[derive(Debug, PartialEq)]
struct SpanData {
    start_idx: usize,
    end_idx: usize,
    code_bold_italic: SpanStyleEnum,
}

#[derive(Debug, Clone, PartialEq)]
enum DelimiterType {
    Null,
    Asterisk,
    Escape,
    UnderScore,
    Tick,
}

impl DelimiterType {
    fn to_char(&self) -> char {
        match self {
            DelimiterType::Null => ' ',
            DelimiterType::Asterisk => '*',
            DelimiterType::Escape => '\\',
            DelimiterType::UnderScore => '_',
            DelimiterType::Tick => '`',
        }
    }
    fn from_char(c: char) -> Self {
        match c {
            '*' => DelimiterType::Asterisk,
            '\\' => DelimiterType::Escape,
            '_' => DelimiterType::UnderScore,
            '`' => DelimiterType::Tick,
            _ => DelimiterType::Null,
        }
    }
}

// based on the commonmark implementation https://spec.commonmark.org/0.31.2/
#[derive(Debug, Clone)]
struct DelimiterData {
    ind: usize, // goes just before this char
    char_type: DelimiterType,
    num_delimiters: usize,
    is_active: bool,
    could_open: bool,
    could_close: bool,
}

fn md_parse_delimiters(text_value: &str) -> (Vec<String>, Vec<DelimiterData>) {
    let mut result = vec![];
    let mut delimiters = vec![DelimiterData {
        ind: 0,
        char_type: DelimiterType::Null,
        num_delimiters: 0,
        is_active: false,
        could_open: false,
        could_close: false,
    }];

    let mut current_span = String::new();

    // first pass find delimiters
    for c in text_value.chars() {
        match DelimiterType::from_char(c) {
            DelimiterType::Null => current_span.push(c),
            // the delimiters and escape
            _ => {
                if !current_span.is_empty() {
                    result.push(current_span);
                    current_span = String::new();
                }
                let last = delimiters.last_mut().expect("guaranteed not to be empty");
                if DelimiterType::from_char(c) == last.char_type && last.ind == result.len() {
                    // is a continuation
                    last.num_delimiters += 1;
                } else {
                    delimiters.push(DelimiterData {
                        ind: result.len(),
                        char_type: DelimiterType::from_char(c),
                        num_delimiters: 1,
                        is_active: true,
                        could_open: true,
                        could_close: true,
                    });
                }
            }
        }
    }
    if !current_span.is_empty() {
        result.push(current_span);
    }

    (result, delimiters)
}

fn md_parse_code_blocks(
    result: Vec<String>,
    delimiters: &mut Vec<DelimiterData>,
) -> (Vec<String>, Vec<SpanData>) {
    let mut new_result = vec![];
    let mut spans = vec![];
    let mut res_ind = 0;
    let mut del_ind = 0;
    let mut removed_spans = 0;

    let mut current_span = String::new();
    while res_ind <= result.len() {
        while del_ind < delimiters.len() && delimiters[del_ind].ind <= res_ind {
            if delimiters[del_ind].char_type == DelimiterType::Tick {
                // if previous delimiter is \ and is right before and is odd number of \
                // then reduce by 1 and re_add it if it does make a pair
                // need to acount for all previous delimiters have been moved by re_added letters
                let escaped = del_ind != 0
                    && delimiters[del_ind - 1].ind + removed_spans == delimiters[del_ind].ind
                    && delimiters[del_ind - 1].char_type == DelimiterType::Escape
                    && delimiters[del_ind - 1].num_delimiters % 2 != 0;
                let needed_len = match escaped {
                    false => delimiters[del_ind].num_delimiters,
                    true => delimiters[del_ind].num_delimiters - 1,
                };

                for closer_ind in (del_ind + 1)..delimiters.len() {
                    if delimiters[del_ind].char_type == delimiters[closer_ind].char_type
                        && delimiters[closer_ind].num_delimiters == needed_len
                    {
                        // it is a span

                        // disable both
                        delimiters[del_ind].is_active = false;
                        delimiters[closer_ind].is_active = false;
                        delimiters[del_ind].num_delimiters = 0;
                        delimiters[closer_ind].num_delimiters = 0;

                        // readd escaped tick
                        if escaped {
                            delimiters[del_ind - 1].num_delimiters -= 1;
                            current_span.push(delimiters[del_ind].char_type.to_char());
                        }

                        if !current_span.is_empty() {
                            new_result.push(current_span);
                            current_span = String::new();
                        }

                        let start_ind = new_result.len();

                        // to make easy to remove edge spaces if any
                        let mut has_none_space = false;

                        del_ind += 1;
                        while res_ind <= delimiters[closer_ind].ind {
                            while del_ind < closer_ind && delimiters[del_ind].ind <= res_ind {
                                // if there is a delimiter it will not be a space
                                has_none_space |= delimiters[del_ind].num_delimiters != 0;

                                // readd delimiter literal
                                current_span += &delimiters[del_ind]
                                    .char_type
                                    .to_char()
                                    .to_string()
                                    .repeat(delimiters[del_ind].num_delimiters);
                                // mark for removal
                                delimiters[del_ind].num_delimiters = 0;
                                del_ind += 1;
                            }
                            if res_ind != delimiters[closer_ind].ind {
                                // one fewer span
                                removed_spans += 1;
                                // check if has non space
                                has_none_space |= result[res_ind].contains(|c| c != ' ');
                                // merge
                                current_span += &result[res_ind];
                                res_ind += 1;
                            } else {
                                break;
                            }
                        }

                        // if the span starts and ends with a space
                        // and is not all space then remove first and last space
                        if has_none_space
                            && current_span.len() > 1
                            && current_span.starts_with(' ')
                            && current_span.ends_with(' ')
                        {
                            current_span = current_span[1..current_span.len() - 1].to_string();
                            // chop off each end
                        }
                        // adding a new span
                        if !current_span.is_empty() {
                            removed_spans -= 1;
                            new_result.push(current_span);
                            current_span = String::new();
                        }

                        let end_ind = new_result.len();

                        // add style span data
                        spans.push(SpanData {
                            start_idx: start_ind,
                            end_idx: end_ind,
                            code_bold_italic: SpanStyleEnum::Code,
                        });

                        break;
                    }
                }
            }
            delimiters[del_ind].ind -= removed_spans;

            del_ind += 1;
        }

        if res_ind != result.len() {
            new_result.push(result[res_ind].clone());
        }

        res_ind += 1;
    }

    // remove all 0 length delimiters except for the null one
    delimiters[0].num_delimiters = 1;
    delimiters.retain(|d| d.num_delimiters != 0);
    delimiters[0].num_delimiters = 0;

    (new_result, spans)
}

fn md_parse_escapes(
    result: Vec<String>,
    delimiters: &mut [DelimiterData],
) -> (Vec<String>, Vec<DelimiterData>) {
    let mut new_result = vec![];
    let mut new_delimiters = vec![];
    let mut added_spans = 0;
    let mut del_ind = 0;
    let mut res_ind = 0;

    let mut current_span = String::new();
    while res_ind <= result.len() {
        while del_ind < delimiters.len() && delimiters[del_ind].ind <= res_ind {
            match delimiters[del_ind].char_type {
                DelimiterType::Escape => {
                    if !current_span.is_empty() && delimiters[del_ind].num_delimiters != 0 {
                        new_result.push(current_span);
                        current_span = String::new();
                    }

                    // readd 1 '\' for every 2 rounded down
                    current_span += &delimiters[del_ind]
                        .char_type
                        .to_char()
                        .to_string()
                        .repeat(delimiters[del_ind].num_delimiters / 2);

                    // if escapes dont all cancel out
                    if delimiters[del_ind].num_delimiters % 2 != 0 {
                        if del_ind != delimiters.len() - 1
                            && delimiters[del_ind + 1].ind == delimiters[del_ind].ind
                        {
                            match delimiters[del_ind + 1].char_type {
                                DelimiterType::Tick
                                | DelimiterType::Asterisk
                                | DelimiterType::UnderScore => {
                                    added_spans += 1;
                                    current_span.push(delimiters[del_ind + 1].char_type.to_char());
                                    delimiters[del_ind + 1].num_delimiters -= 1;
                                }
                                // escapes if adjacent should merge and null is only first
                                _ => panic!("\\ => should merge"),
                            }
                        } else {
                            // letter specific values
                            match result[delimiters[del_ind].ind].chars().next() {
                                Some('n') => {
                                    current_span.push('\n');
                                    current_span += &result[delimiters[del_ind].ind][1..];
                                    res_ind += 1;
                                    delimiters[del_ind].num_delimiters -= 1;
                                }
                                // not escapable so put \ back
                                _ => {
                                    current_span.push(delimiters[del_ind].char_type.to_char());
                                    current_span += &result[delimiters[del_ind].ind];
                                    res_ind += 1;
                                    delimiters[del_ind].num_delimiters -= 1;
                                }
                            }
                        }
                    }
                }
                DelimiterType::Tick => {
                    // unused from previous stage readd to string
                    current_span += &delimiters[del_ind]
                        .char_type
                        .to_char()
                        .to_string()
                        .repeat(delimiters[del_ind].num_delimiters);
                }
                DelimiterType::Null | DelimiterType::Asterisk | DelimiterType::UnderScore => {
                    // future stages assume no 0 len delimiters
                    if delimiters[del_ind].char_type == DelimiterType::Null
                        || delimiters[del_ind].num_delimiters != 0
                    {
                        new_delimiters.push(delimiters[del_ind].clone());
                        let last_ind = new_delimiters.len() - 1;
                        new_delimiters[last_ind].ind += added_spans;
                    }
                }
            }

            del_ind += 1;
        }
        if !current_span.is_empty() {
            new_result.push(current_span);
            current_span = String::new();
        }

        if res_ind != result.len() {
            current_span += &result[res_ind].clone();
        }
        res_ind += 1;
    }

    if !current_span.is_empty() {
        new_result.push(current_span);
    }

    (new_result, new_delimiters)
}

fn md_parse_set_delimiter_open_close(result: &[String], delimiters: &mut [DelimiterData]) {
    // set could open/close
    for i in 0..delimiters.len() {
        // find which char was before and after it
        let prev_char;
        let next_char;
        if i != 0 && delimiters[i - 1].ind == delimiters[i].ind {
            prev_char = delimiters[i - 1].char_type.to_char();
        } else if delimiters[i].ind == 0 {
            prev_char = ' ';
        } else {
            prev_char = result[delimiters[i].ind - 1]
                .chars()
                .last()
                .expect("no 0 len spans");
        }

        if i != delimiters.len() - 1 && delimiters[i + 1].ind == delimiters[i].ind {
            next_char = delimiters[i + 1].char_type.to_char();
        } else if delimiters[i].ind == result.len() {
            next_char = ' ';
        } else {
            next_char = result[delimiters[i].ind]
                .chars()
                .next()
                .expect("no 0 len spans");
        }

        // if prev is whitespace cant end
        // if next is whitespace cant start
        // if neither whitespace but is underscore then cant either
        match (prev_char.is_whitespace(), next_char.is_whitespace()) {
            (false, false) => {
                if delimiters[i].char_type == DelimiterType::UnderScore {
                    delimiters[i].could_open = false;
                    delimiters[i].could_close = false;
                }
            }
            (true, false) => {
                delimiters[i].could_close = false;
            }
            (false, true) => {
                delimiters[i].could_open = false;
            }
            (true, true) => {
                delimiters[i].could_open = false;
                delimiters[i].could_close = false;
            }
        }

        // if next is punctuation and prev is alphanumeric then cant start
        // oposite for end
        if next_char.is_ascii_punctuation()
            && (!prev_char.is_whitespace() || !prev_char.is_ascii_punctuation())
        {
            delimiters[i].could_open = false;
        }
        if prev_char.is_ascii_punctuation()
            && (!next_char.is_whitespace() || !next_char.is_ascii_punctuation())
        {
            delimiters[i].could_close = false;
        }
    }
}

fn md_parse_eval_spans(delimiters: &mut [DelimiterData]) -> Vec<SpanData> {
    let mut spans = vec![];
    let stack_bottom = 0; // because I have a null element in it
    let mut current_position = stack_bottom + 1;
    let mut opener_a = [stack_bottom; 3];
    let mut opener_d = [stack_bottom; 3];

    loop {
        // find something which can close
        while current_position != delimiters.len()
            && !delimiters[current_position].could_close
            && delimiters[current_position].is_active
        {
            current_position += 1;
        }
        if current_position == delimiters.len() {
            break;
        }
        // check which type it is
        let opener_min = match delimiters[current_position].char_type {
            DelimiterType::Asterisk => &mut opener_a,
            DelimiterType::UnderScore => &mut opener_d,
            _ => panic!("this cant happen as current_position starts at 0 and all remaining delimiters are of above types"),
        };

        // min is the value upto which has already been checked for this type
        let min = opener_min[delimiters[current_position].num_delimiters % 3].max(stack_bottom);

        // go down from the previous until at min
        let mut opener_ind = current_position - 1;
        while opener_ind > min {
            // found opener
            if delimiters[opener_ind].is_active
                && delimiters[opener_ind].could_open
                && delimiters[opener_ind].char_type == delimiters[current_position].char_type
                // see spec
                // if one of them could open and close then sum cant be multiple of 3 unless both are
                && !((delimiters[opener_ind].could_close
                    || delimiters[current_position].could_open)
                    && delimiters[opener_ind].num_delimiters % 3
                        != delimiters[current_position].num_delimiters % 3)
            {
                // found valid opener
                break;
            }
            opener_ind -= 1;
        }

        // if hit min then there was no valid one
        if opener_ind == min {
            // update checked upto point
            opener_min[delimiters[current_position].num_delimiters % 3] = current_position - 1;
            current_position += 1;
        } else {
            // a delimiter cant both open and close
            delimiters[current_position].could_open = false;
            delimiters[opener_ind].could_close = false;

            // it is strong emphasis if both have more than 2 delimiters
            let strong = delimiters[opener_ind].num_delimiters >= 2
                && delimiters[current_position].num_delimiters >= 2;
            // create style data for this
            spans.push(SpanData {
                start_idx: delimiters[opener_ind].ind,
                end_idx: delimiters[current_position].ind,
                code_bold_italic: match strong {
                    true => SpanStyleEnum::Bold,
                    false => SpanStyleEnum::Italic,
                },
            });

            // decrease remaining delimiters
            delimiters[opener_ind].num_delimiters -= 1 + (strong as usize);
            delimiters[current_position].num_delimiters -= 1 + (strong as usize);

            // if go to 0 deactivate
            if delimiters[opener_ind].num_delimiters == 0 {
                delimiters[opener_ind].is_active = false;
            }
            if delimiters[current_position].num_delimiters == 0 {
                delimiters[current_position].is_active = false;
                current_position += 1;
            }

            // deactiveate all delimiters inside the new style span
            for d in &mut delimiters[(opener_ind + 1)..current_position] {
                d.is_active = false;
            }
        }
    }
    spans
}

fn md_parse(text_value: &str) -> (Vec<String>, Vec<SpanData>) {
    // parse the string into a vec of strings which are seperated by delimiters
    // delimiters starts with a null delimiter and is always sorted by ind
    // there are no 0 length delimiters except for the first null one
    let (result, mut delimiters) = md_parse_delimiters(text_value);

    // parse code blocks and any escapes directly effecting it
    // code blocks have the highest precedence
    let (result, mut span_data) = md_parse_code_blocks(result, &mut delimiters);

    // other escapes are parsed and remaining ticks are reinserted
    // after this point there should be no ticks or escapes in delimiters
    let (mut result, mut delimiters) = md_parse_escapes(result, &mut delimiters);

    // for each remaining delimiter it is checked whether
    // it could start or end the style
    md_parse_set_delimiter_open_close(&result, &mut delimiters);

    // the delimiters are parsed and evaluated which ones form
    // pairs and create style regions
    // this does not remove delimiters which are fully used
    // so after this 0 length delimiters can exist
    span_data.append(&mut md_parse_eval_spans(&mut delimiters));

    let mut final_result = vec![];

    // all remaining delimiters are reinserted into the string as seperate spans
    for d in delimiters.into_iter().rev() {
        while d.ind < result.len() {
            if let Some(thing) = result.pop() {
                final_result.push(thing);
            }
        }

        // update style regions
        if d.char_type != DelimiterType::Null && d.num_delimiters != 0 {
            for s in span_data.iter_mut() {
                // if start needs to be after or equal
                if s.start_idx >= d.ind {
                    s.start_idx += 1;
                }
                if s.end_idx > d.ind {
                    // if end needs to be after
                    s.end_idx += 1;
                }
            }

            final_result.push(d.char_type.to_char().to_string().repeat(d.num_delimiters));
        }
    }

    (final_result.into_iter().rev().collect(), span_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md_parse() {
        // the basic examples no actual md

        let text = r"Hello, \nworld!";
        assert_eq!(md_parse(text).0, ["Hello, ", "\nworld!"]);
        assert_eq!(md_parse(text).1, []);

        // when not part of a '\n', '\' is not special
        let text = r"Hello, world! \1";
        assert_eq!(md_parse(text).0, ["Hello, world! ", "\\1"]);
        assert_eq!(md_parse(text).1, []);

        // when precedes '\n', '\' escapes it.
        let text = r"Hello, \\nworld!";
        assert_eq!(md_parse(text).0, ["Hello, ", "\\", "nworld!"]);
        assert_eq!(md_parse(text).1, []);

        fn sd(s: usize, e: usize, i: u8) -> SpanData {
            SpanData {
                start_idx: s,
                end_idx: e,
                code_bold_italic: match i {
                    0 => SpanStyleEnum::Code,
                    1 => SpanStyleEnum::Bold,
                    2 => SpanStyleEnum::Italic,
                    _ => unreachable!("set by tests to only be 0 1 or 2"),
                },
            }
        }

        // using the md
        let text = r"He*ll*o, \nworld!";
        assert_eq!(md_parse(text).0, ["He", "ll", "o, ", "\nworld!"]);
        assert_eq!(md_parse(text).1, [sd(1, 2, 2)]);

        // mismatched
        let text = r"*Hello** , \nworld!";
        assert_eq!(md_parse(text).0, ["Hello", "*", " , ", "\nworld!"]);
        assert_eq!(md_parse(text).1, [sd(0, 1, 2)]);

        // diff type
        let text = r"He*llo_, \nworld!";
        assert_eq!(md_parse(text).0, ["He", "*", "llo", "_", ", ", "\nworld!"]);
        assert_eq!(md_parse(text).1, []);

        // multiple diff type
        let text = r"_hello*";
        assert_eq!(md_parse(text).0, ["_", "hello", "*"]);
        assert_eq!(md_parse(text).1, []);

        // multiple same type
        let text = r"He*ll*o, \nw*or*ld!";
        assert_eq!(md_parse(text).0, ["He", "ll", "o, ", "\nw", "or", "ld!"]);
        assert_eq!(md_parse(text).1, [sd(1, 2, 2), sd(4, 5, 2)]);

        // space before
        let text = r"**foo bar **";
        assert_eq!(md_parse(text).0, ["**", "foo bar ", "**"]);
        assert_eq!(md_parse(text).1, []);

        // punctuation before alphnum after
        let text = r"**(**foo)";
        assert_eq!(md_parse(text).0, ["**", "(", "**", "foo)"]);
        assert_eq!(md_parse(text).1, []);
    }

    #[test]
    fn test_get_md_value() {
        fn tc(i: u32, t: &str) -> MdSpan {
            MdSpan {
                code: i & (1 << 0) != 0,
                bold: i & (1 << 1) != 0,
                italic: i & (1 << 2) != 0,
                text: t.to_string(),
            }
        }

        let mut el = SvgElement::new("text", &[]);
        let text = r"foo";
        el.set_attr("md", text);
        assert_eq!(get_md_value(&mut el), [tc(0, "foo")]);

        let text = r"**(**foo)";
        el.set_attr("md", text);
        assert_eq!(get_md_value(&mut el), [tc(0, "**(**foo)")]);

        let text = r"*foo *bar**";
        el.set_attr("md", text);
        assert_eq!(get_md_value(&mut el), [tc(4, "foo bar")]);

        let text = r"*foo**bar**baz*";
        el.set_attr("md", text);
        assert_eq!(
            get_md_value(&mut el),
            [tc(4, "foo"), tc(6, "bar"), tc(4, "baz")]
        );

        let text = r"`foo*`";
        el.set_attr("md", text);
        assert_eq!(get_md_value(&mut el), [tc(1, "foo*")]);

        // if first and last chars in code block are space remove them unless all empty
        let text = r"` `` `";
        el.set_attr("md", text);
        assert_eq!(get_md_value(&mut el), [tc(1, "``")]);

        let text = r"`  `";
        el.set_attr("md", text);
        assert_eq!(get_md_value(&mut el), [tc(1, "  ")]);
    }
}
