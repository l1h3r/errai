use parser::id3v2::Header;
use parser::id3v2::HeaderFlags;
use parser::types::Version;
use std::io::Cursor;

#[test]
fn test_parse_header_v2() {
  let buffer: &[u8] = &[b'I', b'D', b'3', 0x02, 0x00, 0x00, 0x00, 0x00, 0x02, 0x01];
  let cursor: Cursor<&[u8]> = Cursor::new(buffer);
  let header: Header = Header::from_reader(cursor).unwrap();

  assert_eq!(header.version(), Version::ID3v22);
  assert_eq!(header.bitflags(), HeaderFlags::empty());
  assert_eq!(header.data_len(), 257);
  assert_eq!(header.exheader(), None);
}

#[test]
fn test_parse_header_v3() {
  let buffer: &[u8] = &[b'I', b'D', b'3', 0x03, 0x00, 0x00, 0x00, 0x00, 0x02, 0x01];
  let cursor: Cursor<&[u8]> = Cursor::new(buffer);
  let header: Header = Header::from_reader(cursor).unwrap();

  assert_eq!(header.version(), Version::ID3v23);
  assert_eq!(header.bitflags(), HeaderFlags::empty());
  assert_eq!(header.data_len(), 257);
  assert_eq!(header.exheader(), None);
}

#[test]
fn test_parse_header_v4() {
  let buffer: &[u8] = &[b'I', b'D', b'3', 0x04, 0x00, 0x00, 0x00, 0x00, 0x02, 0x01];
  let cursor: Cursor<&[u8]> = Cursor::new(buffer);
  let header: Header = Header::from_reader(cursor).unwrap();

  assert_eq!(header.version(), Version::ID3v24);
  assert_eq!(header.bitflags(), HeaderFlags::empty());
  assert_eq!(header.data_len(), 257);
  assert_eq!(header.exheader(), None);
}
