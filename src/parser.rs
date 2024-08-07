use nom::complete::tag;
use nom::IResult;
use nom::number::complete::be_u16;
use nom::bytes::complete::take;

pub fn length_value(input: &[u8]) -> IResult<&[u8],&[u8]> {
    let (input, length) = be_u16(input)?;
    take(length)(input)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_length_value() {
        let input = [0x00, 0x04, 0x01, 0x02, 0x03, 0x04];
        let (rest, output) = length_value(&input).unwrap();
        assert_eq!(rest, [0x03, 0x04]);
        assert_eq!(output, [0x01, 0x02, 0x03, 0x04]);
    }
}
