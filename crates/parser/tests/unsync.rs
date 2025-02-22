use parser::unsync::Unsync;
use std::io::Cursor;
use std::io::Read;

const INPUT: &[u8] = &[0xFF, 0x00, 0x00, 0x01, 0x02, 0xFF, 0x00, 0x00, 0x03];
const OUTPUT: &[u8] = &[0xFF, 0x00, 0x01, 0x02, 0xFF, 0x00, 0x03];

#[test]
fn test_unsync() {
  let cursor: Cursor<&[u8]> = Cursor::new(INPUT);

  let mut reader: Unsync<_> = Unsync::new(cursor);
  let mut output: Vec<u8> = Vec::new();

  reader.read_to_end(&mut output).unwrap();

  assert_eq!(output, OUTPUT);
}
