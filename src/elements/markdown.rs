use super::SvgElement;

pub fn get_md_value(element: &mut SvgElement) -> (Vec<String>, Vec<TextClass>) {
    let text_value = if let Some(tv) = element.pop_attr("md") {
        tv
    } else {
        element
            .pop_attr("text")
            .expect("no text attr in process_text_attr")
    };

    let (parsed_string, sections) = md_parse(&text_value);

    let mut state_per_char = vec![
        TextClass {
            code: false,
            bold: false,
            italic: false
        };
        parsed_string.len()
    ];

    for s in sections {
        let class = s.code_bold_italic;
        for i in state_per_char.iter_mut().take(s.end_ind).skip(s.start_ind) {
            match class {
                TextClassEnum::Code => i.code = true,
                TextClassEnum::Bold => i.bold = true,
                TextClassEnum::Italic => i.italic = true,
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
pub struct TextClass {
    pub code: bool,
    pub bold: bool,
    pub italic: bool,
}

#[derive(Debug)]
enum TextClassEnum {
    Code,
    Bold,
    Italic,
}

#[derive(Debug)]
struct SectionData {
    start_ind: usize,
    end_ind: usize,
    code_bold_italic: TextClassEnum,
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
                let last = delimiters.last_mut().expect("garenteed not to be empty");
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
) -> (Vec<char>, Vec<SectionData>) {
    let mut new_result = vec![];
    let mut sections = vec![];
    let mut res_ind = 0;
    let mut del_ind = 0;
    let mut readded_letters = 0;
    while res_ind <= result.len() {
        while del_ind < delimiters.len() && delimiters[del_ind].ind <= res_ind {
            if delimiters[del_ind].char_type == '`' {
                // if previous delimiter is \ and is right before and is odd number of \
                // then reduce by 1 and readd it if it does make a pair
                // need to acount for all previous delimiters have been moved by readded letters
                let escaped = del_ind != 0
                    && delimiters[del_ind - 1].ind - readded_letters == delimiters[del_ind].ind
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
                        // it is a section
                        delimiters[del_ind].is_active = false;
                        delimiters[closer_ind].is_active = false;
                        delimiters[del_ind].num_delimiters = 0;
                        delimiters[closer_ind].num_delimiters = 0;

                        if escaped {
                            delimiters[del_ind - 1].num_delimiters -= 1;
                            new_result.push('`');
                            readded_letters += 1;
                        }
                        let start_ind = new_result.len();

                        del_ind += 1;
                        while res_ind <= delimiters[closer_ind].ind {
                            while del_ind < closer_ind && delimiters[del_ind].ind <= res_ind {
                                let mut temp = vec![
                                    delimiters[del_ind].char_type;
                                    delimiters[del_ind].num_delimiters
                                ];
                                new_result.append(&mut temp);
                                readded_letters += delimiters[del_ind].num_delimiters;
                                delimiters[del_ind].num_delimiters = 0;
                                del_ind += 1;
                            }
                            if res_ind != delimiters[closer_ind].ind {
                                new_result.push(result[res_ind]);
                                res_ind += 1;
                            } else {
                                break;
                            }
                        }
                        let end_ind = new_result.len();

                        sections.push(SectionData {
                            start_ind,
                            end_ind,
                            code_bold_italic: TextClassEnum::Code,
                        });

                        break;
                    }
                }
            }
            delimiters[del_ind].ind += readded_letters;

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

    (new_result, sections)
}

fn md_parse_escapes(
    result: &[char],
    delimiters: &mut [DelimiterData],
) -> (Vec<char>, Vec<DelimiterData>) {
    let mut new_result = vec![];
    let mut new_delimiters = vec![];
    let mut readded_letters = 0;
    let mut del_ind = 0;
    let mut res_ind = 0;
    while res_ind <= result.len() {
        while del_ind < delimiters.len() && delimiters[del_ind].ind <= res_ind {
            match delimiters[del_ind].char_type {
                '\\' => {
                    let mut temp =
                        vec![delimiters[del_ind].char_type; delimiters[del_ind].num_delimiters / 2];
                    new_result.append(&mut temp);
                    readded_letters += delimiters[del_ind].num_delimiters / 2;

                    if delimiters[del_ind].num_delimiters % 2 != 0 {
                        if del_ind != delimiters.len() - 1
                            && delimiters[del_ind + 1].ind == delimiters[del_ind].ind
                        {
                            match delimiters[del_ind + 1].char_type {
                                '`' | '*' | '_' => {
                                    new_result.push(delimiters[del_ind + 1].char_type);
                                    readded_letters += 1;
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
                                    readded_letters += 1;
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
                    readded_letters += delimiters[del_ind].num_delimiters;
                }
                ' ' | '*' | '_' => {
                    new_delimiters.push(delimiters[del_ind].clone());
                    let last_ind = new_delimiters.len() - 1;
                    new_delimiters[last_ind].ind += readded_letters;
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

fn md_parse_eval_sections(delimiters: &mut [DelimiterData]) -> Vec<SectionData> {
    let mut sections = vec![];
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
            sections.push(SectionData {
                start_ind: delimiters[opener_ind].ind,
                end_ind: delimiters[current_position].ind,
                code_bold_italic: match (code, strong) {
                    (true, _) => TextClassEnum::Code,
                    (_, true) => TextClassEnum::Bold,
                    (_, _) => TextClassEnum::Italic,
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
    sections
}

fn md_parse(text_value: &str) -> (Vec<char>, Vec<SectionData>) {
    let (result, mut delimiters) = md_parse_delimiters(text_value);
    let (result, mut sections) = md_parse_code_blocks(&result, &mut delimiters);
    let (mut result, mut delimiters) = md_parse_escapes(&result, &mut delimiters);
    md_parse_set_delimiter_open_close(&result, &mut delimiters);
    sections.append(&mut md_parse_eval_sections(&mut delimiters));

    let mut final_result = vec![];

    // work from the back to avoid index invalidation
    for d in delimiters.into_iter().rev() {
        while d.ind < result.len() {
            if let Some(thing) = result.pop() {
                final_result.push(thing);
            }
        }

        for s in sections.iter_mut() {
            // if start needs to be after or equal
            if s.start_ind >= d.ind {
                s.start_ind += d.num_delimiters;
            }
            if s.end_ind > d.ind {
                // if end needs to be after
                s.end_ind += d.num_delimiters;
            }
        }
        if d.char_type != ' ' {
            let mut temp = vec![d.char_type; d.num_delimiters];
            final_result.append(&mut temp);
        }
    }

    (final_result.into_iter().rev().collect(), sections)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md_parse() {
        // the basic examples no actual md

        let text = r"Hello, \nworld!";
        assert_eq!(
            format!("{:?}", md_parse(text)),
            "(['H', 'e', 'l', 'l', 'o', ',', ' ', '\\n', 'w', 'o', 'r', 'l', 'd', '!'], [])"
        );

        // when not part of a '\n', '\' is not special
        let text = r"Hello, world! \1";
        assert_eq!(format!("{:?}",md_parse(text)), "(['H', 'e', 'l', 'l', 'o', ',', ' ', 'w', 'o', 'r', 'l', 'd', '!', ' ', '\\\\', '1'], [])");

        // when precedes '\n', '\' escapes it.
        let text = r"Hello, \\nworld!";
        assert_eq!(
            format!("{:?}", md_parse(text)),
            "(['H', 'e', 'l', 'l', 'o', ',', ' ', '\\\\', 'n', 'w', 'o', 'r', 'l', 'd', '!'], [])"
        );

        fn sd(s: i32, e: i32, i: i32) -> String {
            "SectionData { start_ind: ".to_owned()
                + &s.to_string()
                + ", end_ind: "
                + &e.to_string()
                + ", code_bold_italic: "
                + match i {
                    0 => "Code",
                    1 => "Bold",
                    2 => "Italic",
                    _ => "Err",
                }
                + " }"
        }

        // using the md
        let text = r"He*ll*o, \nworld!";
        assert_eq!(
            format!("{:?}", md_parse(text)),
            "(['H', 'e', 'l', 'l', 'o', ',', ' ', '\\n', 'w', 'o', 'r', 'l', 'd', '!'], ["
                .to_owned()
                + &sd(2, 4, 2)
                + "])"
        );

        // mismatched
        let text = r"*Hello** , \nworld!";
        assert_eq!(
            format!("{:?}", md_parse(text)),
            "(['H', 'e', 'l', 'l', 'o', '*', ' ', ',', ' ', '\\n', 'w', 'o', 'r', 'l', 'd', '!'], ["
                .to_owned()
                + &sd(0, 5, 2) + "])"
        );

        // diff type
        let text = r"He*llo_, \nworld!";
        assert_eq!(format!("{:?}",md_parse(text)), "(['H', 'e', '*', 'l', 'l', 'o', '_', ',', ' ', '\\n', 'w', 'o', 'r', 'l', 'd', '!'], [])");

        // multiple diff type
        let text = r"_hello*";
        assert_eq!(
            format!("{:?}", md_parse(text)),
            "(['_', 'h', 'e', 'l', 'l', 'o', '*'], [])"
        );

        // multiple same type
        let text = r"He*ll*o, \nw*or*ld!";
        assert_eq!(
            format!("{:?}", md_parse(text)),
            "(['H', 'e', 'l', 'l', 'o', ',', ' ', '\\n', 'w', 'o', 'r', 'l', 'd', '!'], ["
                .to_owned()
                + &sd(2, 4, 2)
                + ", "
                + &sd(9, 11, 2)
                + "])"
        );

        // space before
        let text = r"**foo bar **";
        assert_eq!(
            format!("{:?}", md_parse(text)),
            "(['*', '*', 'f', 'o', 'o', ' ', 'b', 'a', 'r', ' ', '*', '*'], [])"
        );

        // punctuation before alphnum after
        let text = r"**(**foo)";
        assert_eq!(
            format!("{:?}", md_parse(text)),
            "(['*', '*', '(', '*', '*', 'f', 'o', 'o', ')'], [])"
        );
    }

    #[test]
    fn test_get_md_value() {
        fn tc(i: u32) -> String {
            "TextClass { code: ".to_owned()
                + match i & (1 << 0) != 0 {
                    false => "false",
                    true => "true",
                }
                + ", bold: "
                + match i & (1 << 1) != 0 {
                    false => "false",
                    true => "true",
                }
                + ", italic: "
                + match i & (1 << 2) != 0 {
                    false => "false",
                    true => "true",
                }
                + " }"
        }

        let mut el = SvgElement::new("text", &[]);
        let text = r"foo";
        el.set_attr("md", text);
        assert_eq!(
            format!("{:?}", get_md_value(&mut el)),
            "([\"foo\"], [".to_owned() + &tc(0) + "])"
        );

        let text = r"**(**foo)";
        el.set_attr("md", text);
        assert_eq!(
            format!("{:?}", get_md_value(&mut el)),
            "([\"**(**foo)\"], [".to_owned() + &tc(0) + "])"
        );

        let text = r"*foo *bar**";
        el.set_attr("md", text);
        assert_eq!(
            format!("{:?}", get_md_value(&mut el)),
            "([\"foo bar\"], [".to_owned() + &tc(4) + "])"
        );

        let text = r"*foo**bar**baz*";
        el.set_attr("md", text);
        assert_eq!(
            format!("{:?}", get_md_value(&mut el)),
            "([\"foo\", \"bar\", \"baz\"], [".to_owned()
                + &tc(4)
                + ", "
                + &tc(6)
                + ", "
                + &tc(4)
                + "])"
        );

        let text = r"`foo*`";
        el.set_attr("md", text);
        assert_eq!(
            format!("{:?}", get_md_value(&mut el)),
            "([\"foo*\"], [".to_owned() + &tc(1) + "])"
        );
    }
}
