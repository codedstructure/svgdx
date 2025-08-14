use super::SvgElement;

pub fn get_md_value(element: &mut SvgElement) -> (Vec<String>, Vec<SpanStyle>) {
    let text_value = if let Some(tv) = element.pop_attr("md") {
        tv
    } else if let Some(tv) = element.pop_attr("text") {
        tv
    } else {
        return (vec![], vec![]);
    };

    let (parsed_string, spans) = md_parse(&text_value);

    let mut state_per_char = vec![
        SpanStyle {
            code: false,
            bold: false,
            italic: false
        };
        parsed_string.len()
    ];

    for s in spans {
        let class = s.code_bold_italic;
        for i in state_per_char.iter_mut().take(s.end_idx).skip(s.start_idx) {
            match class {
                SpanStyleEnum::Code => i.code = true,
                SpanStyleEnum::Bold => i.bold = true,
                SpanStyleEnum::Italic => i.italic = true,
            }
        }
    }

    let mut strings = vec![];
    let mut states = vec![];
    for i in 0..parsed_string.len() {
        if i == 0 || states[states.len() - 1] != state_per_char[i] {
            strings.push(String::new());
            states.push(state_per_char[i])
        }
        strings
            .last_mut()
            .expect("filled from i == 0")
            .push(parsed_string[i]);
    }

    (strings, states)
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct SpanStyle {
    pub code: bool,
    pub bold: bool,
    pub italic: bool,
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

// based on the commonmark implementation https://spec.commonmark.org/0.31.2/
#[derive(Debug, Clone)]
struct DelimiterData {
    ind: usize, // goes just before this char
    char_type: char,
    num_delimiters: usize,
    is_active: bool,
    could_open: bool,
    could_close: bool,
}

fn md_parse_delimiters(text_value: &str) -> (Vec<char>, Vec<DelimiterData>) {
    let mut result = vec![];
    let mut delimiters = vec![DelimiterData {
        ind: 0,
        char_type: ' ',
        num_delimiters: 0,
        is_active: false,
        could_open: false,
        could_close: false,
    }];

    // first pass find delimiters
    for c in text_value.chars() {
        let mut add = true;
        match c {
            // the delimiters and escape
            '`' | '_' | '*' | '\\' => {
                let last = delimiters.last_mut().expect("guarenteed not to be empty");
                if c == last.char_type && last.ind == result.len() {
                    // is a continuation
                    last.num_delimiters += 1;
                } else {
                    delimiters.push(DelimiterData {
                        ind: result.len(),
                        char_type: c,
                        num_delimiters: 1,
                        is_active: true,
                        could_open: true,
                        could_close: true,
                    });
                }
                add = false;
            }
            _ => {}
        }
        if add {
            result.push(c);
        }
    }

    (result, delimiters)
}

// assumes delimiters are ordered
fn md_parse_code_blocks(
    result: &[char],
    delimiters: &mut Vec<DelimiterData>,
) -> (Vec<char>, Vec<SpanData>) {
    let mut new_result = vec![];
    let mut spans = vec![];
    let mut res_ind = 0;
    let mut del_ind = 0;
    let mut re_added_letters = 0;
    while res_ind <= result.len() {
        while del_ind < delimiters.len() && delimiters[del_ind].ind <= res_ind {
            if delimiters[del_ind].char_type == '`' {
                // if previous delimiter is \ and is right before and is odd number of \
                // then reduce by 1 and re_add it if it does make a pair
                // need to acount for all previous delimiters have been moved by re_added letters
                let escaped = del_ind != 0
                    && delimiters[del_ind - 1].ind - re_added_letters == delimiters[del_ind].ind
                    && delimiters[del_ind - 1].char_type == '\\'
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
                        delimiters[del_ind].is_active = false;
                        delimiters[closer_ind].is_active = false;
                        delimiters[del_ind].num_delimiters = 0;
                        delimiters[closer_ind].num_delimiters = 0;

                        if escaped {
                            delimiters[del_ind - 1].num_delimiters -= 1;
                            new_result.push('`');
                            re_added_letters += 1;
                        }
                        let start_ind = new_result.len();

                        // to make easy to remove edge spaces if any
                        let mut has_none_space = false;
                        let mut span_str = vec![];

                        del_ind += 1;
                        while res_ind <= delimiters[closer_ind].ind {
                            while del_ind < closer_ind && delimiters[del_ind].ind <= res_ind {
                                let mut temp = vec![
                                    delimiters[del_ind].char_type;
                                    delimiters[del_ind].num_delimiters
                                ];
                                has_none_space |= delimiters[del_ind].num_delimiters != 0; // char type will not be ' '
                                span_str.append(&mut temp);
                                re_added_letters += delimiters[del_ind].num_delimiters;
                                delimiters[del_ind].num_delimiters = 0;
                                del_ind += 1;
                            }
                            if res_ind != delimiters[closer_ind].ind {
                                has_none_space |= result[res_ind] != ' ';
                                span_str.push(result[res_ind]);
                                res_ind += 1;
                            } else {
                                break;
                            }
                        }
                        if has_none_space
                            && span_str.len() > 1
                            && span_str[0] == ' '
                            && span_str[span_str.len() - 1] == ' '
                        {
                            span_str.pop();
                            let mut itr = span_str.iter();
                            itr.next().expect("size bigger than 1"); // pop front
                            new_result.extend(itr);
                        } else {
                            new_result.append(&mut span_str);
                        }

                        let end_ind = new_result.len();

                        spans.push(SpanData {
                            start_idx: start_ind,
                            end_idx: end_ind,
                            code_bold_italic: SpanStyleEnum::Code,
                        });

                        break;
                    }
                }
            }
            delimiters[del_ind].ind += re_added_letters;

            del_ind += 1;
        }

        if res_ind != result.len() {
            new_result.push(result[res_ind]);
        }

        res_ind += 1;
    }

    delimiters[0].num_delimiters = 1; // set the null delimiter to 1
    delimiters.retain(|d| d.num_delimiters != 0);
    delimiters[0].num_delimiters = 0; // set the null delimiter to 0

    (new_result, spans)
}

// assumes no zero length delimiters except for null delim
// assumes delimiters are ordered
fn md_parse_escapes(
    result: &[char],
    delimiters: &mut [DelimiterData],
) -> (Vec<char>, Vec<DelimiterData>) {
    let mut new_result = vec![];
    let mut new_delimiters = vec![];
    let mut re_added_letters = 0;
    let mut del_ind = 0;
    let mut res_ind = 0;
    while res_ind <= result.len() {
        while del_ind < delimiters.len() && delimiters[del_ind].ind <= res_ind {
            match delimiters[del_ind].char_type {
                '\\' => {
                    let mut temp =
                        vec![delimiters[del_ind].char_type; delimiters[del_ind].num_delimiters / 2];
                    new_result.append(&mut temp);
                    re_added_letters += delimiters[del_ind].num_delimiters / 2;

                    if delimiters[del_ind].num_delimiters % 2 != 0 {
                        if del_ind != delimiters.len() - 1
                            && delimiters[del_ind + 1].ind == delimiters[del_ind].ind
                        {
                            match delimiters[del_ind + 1].char_type {
                                '`' | '*' | '_' => {
                                    new_result.push(delimiters[del_ind + 1].char_type);
                                    re_added_letters += 1;
                                    delimiters[del_ind + 1].num_delimiters -= 1;
                                }
                                _ => panic!("\\ => should merge"),
                            }
                        } else {
                            match result[delimiters[del_ind].ind] {
                                'n' => {
                                    res_ind += 1;
                                    new_result.push('\n');
                                }
                                _ => {
                                    new_result.push(delimiters[del_ind].char_type);
                                    re_added_letters += 1;
                                    delimiters[del_ind].num_delimiters -= 1;
                                }
                            }
                        }
                    }
                }
                '`' => {
                    let mut temp =
                        vec![delimiters[del_ind].char_type; delimiters[del_ind].num_delimiters];
                    new_result.append(&mut temp);
                    re_added_letters += delimiters[del_ind].num_delimiters;
                }
                ' ' | '*' | '_' => {
                    // future stages assume no 0 len delimiters
                    if delimiters[del_ind].char_type == ' '
                        || delimiters[del_ind].num_delimiters != 0
                    {
                        new_delimiters.push(delimiters[del_ind].clone());
                        let last_ind = new_delimiters.len() - 1;
                        new_delimiters[last_ind].ind += re_added_letters;
                    }
                }
                _ => panic!("no other type of delimiter char"),
            }

            del_ind += 1;
        }

        if res_ind != result.len() {
            new_result.push(result[res_ind]);
        }
        res_ind += 1;
    }

    (new_result, new_delimiters)
}

// assumes delimiters are ordered and nonzero
fn md_parse_set_delimiter_open_close(result: &[char], delimiters: &mut [DelimiterData]) {
    // set could open/close
    for i in 0..delimiters.len() {
        let prev_char;
        let next_char;
        if i != 0 && delimiters[i - 1].ind == delimiters[i].ind {
            prev_char = delimiters[i - 1].char_type;
        } else if delimiters[i].ind == 0 {
            prev_char = ' ';
        } else {
            prev_char = result[delimiters[i].ind - 1];
        }

        if i != delimiters.len() - 1 && delimiters[i + 1].ind == delimiters[i].ind {
            next_char = delimiters[i + 1].char_type;
        } else if delimiters[i].ind == result.len() {
            next_char = ' ';
        } else {
            next_char = result[delimiters[i].ind];
        }

        match (prev_char.is_whitespace(), next_char.is_whitespace()) {
            (false, false) => {
                if delimiters[i].char_type == '_' {
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
        while current_position != delimiters.len()
            && !delimiters[current_position].could_close
            && delimiters[current_position].is_active
        {
            current_position += 1;
        }
        if current_position == delimiters.len() {
            break;
        }
        let opener_min = match delimiters[current_position].char_type {
            '*' => &mut opener_a,
            '_' => &mut opener_d,
            _ => panic!("this cant happen as current_position starts at 0 and all other delimiters are of above types"),
        };

        let min = opener_min[delimiters[current_position].num_delimiters % 3].max(stack_bottom);
        let mut opener_ind = current_position - 1;
        while opener_ind > min {
            // found opener
            if delimiters[opener_ind].is_active
                && delimiters[opener_ind].could_open
                && delimiters[opener_ind].char_type == delimiters[current_position].char_type
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

        if opener_ind == min {
            // not found a opener
            opener_min[delimiters[current_position].num_delimiters % 3] = current_position - 1;
            current_position += 1;
        } else {
            delimiters[current_position].could_open = false;
            delimiters[opener_ind].could_close = false;
            // did
            let code = delimiters[current_position].char_type == '`';
            let strong = !code
                && delimiters[opener_ind].num_delimiters >= 2
                && delimiters[current_position].num_delimiters >= 2;
            spans.push(SpanData {
                start_idx: delimiters[opener_ind].ind,
                end_idx: delimiters[current_position].ind,
                code_bold_italic: match (code, strong) {
                    (true, _) => SpanStyleEnum::Code,
                    (_, true) => SpanStyleEnum::Bold,
                    (_, _) => SpanStyleEnum::Italic,
                },
            });

            delimiters[opener_ind].num_delimiters -= 1 + (strong as usize);
            delimiters[current_position].num_delimiters -= 1 + (strong as usize);

            if delimiters[opener_ind].num_delimiters == 0 {
                delimiters[opener_ind].is_active = false;
            }
            if delimiters[current_position].num_delimiters == 0 {
                delimiters[current_position].is_active = false;
                current_position += 1;
            }

            for d in &mut delimiters[(opener_ind + 1)..current_position] {
                d.is_active = false;
            }
        }
    }
    spans
}

fn md_parse(text_value: &str) -> (Vec<char>, Vec<SpanData>) {
    let (result, mut delimiters) = md_parse_delimiters(text_value);
    let (result, mut spans) = md_parse_code_blocks(&result, &mut delimiters);
    let (mut result, mut delimiters) = md_parse_escapes(&result, &mut delimiters);
    md_parse_set_delimiter_open_close(&result, &mut delimiters);
    spans.append(&mut md_parse_eval_spans(&mut delimiters));

    let mut final_result = vec![];

    // work from the back to avoid index invalidation
    for d in delimiters.into_iter().rev() {
        while d.ind < result.len() {
            if let Some(thing) = result.pop() {
                final_result.push(thing);
            }
        }

        for s in spans.iter_mut() {
            // if start needs to be after or equal
            if s.start_idx >= d.ind {
                s.start_idx += d.num_delimiters;
            }
            if s.end_idx > d.ind {
                // if end needs to be after
                s.end_idx += d.num_delimiters;
            }
        }
        if d.char_type != ' ' {
            let mut temp = vec![d.char_type; d.num_delimiters];
            final_result.append(&mut temp);
        }
    }

    (final_result.into_iter().rev().collect(), spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md_parse() {
        // the basic examples no actual md

        let text = r"Hello, \nworld!";
        assert_eq!(
            md_parse(text).0,
            ['H', 'e', 'l', 'l', 'o', ',', ' ', '\n', 'w', 'o', 'r', 'l', 'd', '!']
        );
        assert_eq!(md_parse(text).1, []);

        // when not part of a '\n', '\' is not special
        let text = r"Hello, world! \1";
        assert_eq!(
            md_parse(text).0,
            ['H', 'e', 'l', 'l', 'o', ',', ' ', 'w', 'o', 'r', 'l', 'd', '!', ' ', '\\', '1']
        );
        assert_eq!(md_parse(text).1, []);

        // when precedes '\n', '\' escapes it.
        let text = r"Hello, \\nworld!";
        assert_eq!(
            md_parse(text).0,
            ['H', 'e', 'l', 'l', 'o', ',', ' ', '\\', 'n', 'w', 'o', 'r', 'l', 'd', '!']
        );
        assert_eq!(md_parse(text).1, []);

        fn sd(s: usize, e: usize, i: u8) -> SpanData {
            SpanData {
                start_idx: s,
                end_idx: e,
                code_bold_italic: match i {
                    0 => SpanStyleEnum::Code,
                    1 => SpanStyleEnum::Bold,
                    2 => SpanStyleEnum::Italic,
                    _ => panic!(),
                },
            }
        }

        // using the md
        let text = r"He*ll*o, \nworld!";
        assert_eq!(
            md_parse(text).0,
            ['H', 'e', 'l', 'l', 'o', ',', ' ', '\n', 'w', 'o', 'r', 'l', 'd', '!']
        );
        assert_eq!(md_parse(text).1, [sd(2, 4, 2)]);

        // mismatched
        let text = r"*Hello** , \nworld!";
        assert_eq!(
            md_parse(text).0,
            ['H', 'e', 'l', 'l', 'o', '*', ' ', ',', ' ', '\n', 'w', 'o', 'r', 'l', 'd', '!']
        );
        assert_eq!(md_parse(text).1, [sd(0, 5, 2)]);

        // diff type
        let text = r"He*llo_, \nworld!";
        assert_eq!(
            md_parse(text).0,
            ['H', 'e', '*', 'l', 'l', 'o', '_', ',', ' ', '\n', 'w', 'o', 'r', 'l', 'd', '!']
        );
        assert_eq!(md_parse(text).1, []);

        // multiple diff type
        let text = r"_hello*";
        assert_eq!(md_parse(text).0, ['_', 'h', 'e', 'l', 'l', 'o', '*']);
        assert_eq!(md_parse(text).1, []);

        // multiple same type
        let text = r"He*ll*o, \nw*or*ld!";
        assert_eq!(
            md_parse(text).0,
            ['H', 'e', 'l', 'l', 'o', ',', ' ', '\n', 'w', 'o', 'r', 'l', 'd', '!']
        );
        assert_eq!(md_parse(text).1, [sd(2, 4, 2), sd(9, 11, 2)]);

        // space before
        let text = r"**foo bar **";
        assert_eq!(
            md_parse(text).0,
            ['*', '*', 'f', 'o', 'o', ' ', 'b', 'a', 'r', ' ', '*', '*']
        );
        assert_eq!(md_parse(text).1, []);

        // punctuation before alphnum after
        let text = r"**(**foo)";
        assert_eq!(
            md_parse(text).0,
            ['*', '*', '(', '*', '*', 'f', 'o', 'o', ')']
        );
        assert_eq!(md_parse(text).1, []);
    }

    #[test]
    fn test_get_md_value() {
        fn tc(i: u32) -> SpanStyle {
            SpanStyle {
                code: i & (1 << 0) != 0,
                bold: i & (1 << 1) != 0,
                italic: i & (1 << 2) != 0,
            }
        }

        let mut el = SvgElement::new("text", &[]);
        let text = r"foo";
        el.set_attr("md", text);
        assert_eq!(get_md_value(&mut el).0, ["foo"]);
        el.set_attr("md", text);
        assert_eq!(get_md_value(&mut el).1, [tc(0)]);

        let text = r"**(**foo)";
        el.set_attr("md", text);
        assert_eq!(get_md_value(&mut el).0, ["**(**foo)"]);
        el.set_attr("md", text);
        assert_eq!(get_md_value(&mut el).1, [tc(0)]);

        let text = r"*foo *bar**";
        el.set_attr("md", text);
        assert_eq!(get_md_value(&mut el).0, ["foo bar"]);
        el.set_attr("md", text);
        assert_eq!(get_md_value(&mut el).1, [tc(4)]);

        let text = r"*foo**bar**baz*";
        el.set_attr("md", text);
        assert_eq!(get_md_value(&mut el).0, ["foo", "bar", "baz"]);
        el.set_attr("md", text);
        assert_eq!(get_md_value(&mut el).1, [tc(4), tc(6), tc(4)]);

        let text = r"`foo*`";
        el.set_attr("md", text);
        assert_eq!(get_md_value(&mut el).0, ["foo*"]);
        el.set_attr("md", text);
        assert_eq!(get_md_value(&mut el).1, [tc(1)]);

        // if first and last chars in code block are space remove them unless all empty
        let text = r"` `` `";
        el.set_attr("md", text);
        assert_eq!(get_md_value(&mut el).0, ["``"]);
        el.set_attr("md", text);
        assert_eq!(get_md_value(&mut el).1, [tc(1)]);

        let text = r"`  `";
        el.set_attr("md", text);
        assert_eq!(get_md_value(&mut el).0, ["  "]);
        el.set_attr("md", text);
        assert_eq!(get_md_value(&mut el).1, [tc(1)]);
    }
}
