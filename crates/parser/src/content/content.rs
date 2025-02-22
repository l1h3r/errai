use crate::content::Aenc;
use crate::content::Apic;
use crate::content::Atxt;
use crate::content::Chap;
use crate::content::Comm;
use crate::content::Comr;
use crate::content::Ctoc;
use crate::content::Encr;
use crate::content::Equa;
use crate::content::Etco;
use crate::content::Geob;
use crate::content::Grid;
use crate::content::Ipls;
use crate::content::Link;
use crate::content::Mcdi;
use crate::content::Mllt;
use crate::content::Owne;
use crate::content::Pcnt;
use crate::content::Popm;
use crate::content::Poss;
use crate::content::Priv;
use crate::content::Rbuf;
use crate::content::Rva2;
use crate::content::Rvad;
use crate::content::Rvrb;
use crate::content::Sylt;
use crate::content::Sytc;
use crate::content::Text;
use crate::content::Txxx;
use crate::content::Ufid;
use crate::content::Unkn;
use crate::content::User;
use crate::content::Uslt;
use crate::content::Wurl;
use crate::content::Wxxx;
use crate::decode::Decoder;
use crate::error::Result;
use crate::traits::IntoOwned;
use crate::types::Bytes;
use crate::types::Slice;
use crate::types::Version;
use crate::utils;

// =============================================================================
// Content
// =============================================================================

/// Decoded frame content.
#[derive(Clone, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Content<'a> {
  /// Audio encryption.
  Aenc(Aenc<'a>),
  /// Attached picture.
  Apic(Apic<'a>),
  /// Audio text.
  Atxt(Atxt<'a>),
  /// Chapter.
  Chap(Chap<'a>),
  /// Comments.
  Comm(Comm<'a>),
  /// Table of contents.
  Ctoc(Ctoc<'a>),
  /// Commercial frame.
  Comr(Comr<'a>),
  /// Encryption method registration.
  Encr(Encr<'a>),
  /// Equalization.
  Equa(Equa<'a>),
  /// Event timing codes.
  Etco(Etco<'a>),
  /// General encapsulated object.
  Geob(Geob<'a>),
  /// Group identification registration.
  Grid(Grid<'a>),
  /// Involved people list.
  Ipls(Ipls<'a>),
  /// Linked information.
  Link(Link<'a>),
  /// Music CD identifier.
  Mcdi(Mcdi<'a>),
  /// MPEG location lookup table.
  Mllt(Mllt<'a>),
  /// Ownership frame.
  Owne(Owne<'a>),
  /// Play counter.
  Pcnt(Pcnt),
  /// Popularimeter.
  Popm(Popm<'a>),
  /// Position synchronisation frame.
  Poss(Poss<'a>),
  /// Private.
  Priv(Priv<'a>),
  /// Recommended buffer size.
  Rbuf(Rbuf),
  /// Relative volume adjustment (2).
  Rva2(Rva2<'a>),
  /// Relative volume adjustment.
  Rvad(Rvad<'a>),
  /// Reverb.
  Rvrb(Rvrb),
  /// Synchronised lyrics.
  Sylt(Sylt<'a>),
  /// Synchronized tempo codes.
  Sytc(Sytc<'a>),
  /// Text information.
  Text(Text<'a>),
  /// User-defined text information.
  Txxx(Txxx<'a>),
  /// Unique file identifier.
  Ufid(Ufid<'a>),
  /// Terms of use.
  User(User<'a>),
  /// Unsychronised lyrics.
  Uslt(Uslt<'a>),
  /// Commercial information.
  Wcom(Wurl<'a>),
  /// Copyright/Legal information.
  Wcop(Wurl<'a>),
  /// Official audio file webpage.
  Woaf(Wurl<'a>),
  /// Official artist/performer webpage.
  Woar(Wurl<'a>),
  /// Official audio source webpage.
  Woas(Wurl<'a>),
  /// fficial internet radio station homepage.
  Wors(Wurl<'a>),
  /// Payment.
  Wpay(Wurl<'a>),
  /// Publishers official webpage.
  Wpub(Wurl<'a>),
  /// User-defined URL link frame.
  Wxxx(Wxxx<'a>),
  /// Unknown frame content.
  Unkn(Unkn<'a>),
}

impl<'a> Content<'a> {
  /// Decode a slice of bytes with the format specified by `name`.
  pub fn decode(version: Version, name: &str, slice: &'a Slice) -> Result<Self> {
    let mut decoder: Decoder<'_> = Decoder::new(slice);

    let this: Result<Self> = match (version, name) {
      (Version::ID3v11, _) => panic!("Invalid Version: ID3v11"),
      (Version::ID3v12, _) => panic!("Invalid Version: ID3v12"),
      // =======================================================================
      // ID3v2.2 Frames
      // =======================================================================
      (Version::ID3v22, "BUF") => decoder.decode_v2().map(Self::Rbuf), // Recommended buffer size
      (Version::ID3v22, "CNT") => decoder.decode_v2().map(Self::Pcnt), // Play counter
      (Version::ID3v22, "COM") => decoder.decode_v2().map(Self::Comm), // Comments
      (Version::ID3v22, "CRA") => decoder.decode_v2().map(Self::Aenc), // Audio encryption
      (Version::ID3v22, "CRM") => panic!("TODO: Decode CRM"),          // Encrypted meta frame
      (Version::ID3v22, "ETC") => decoder.decode_v2().map(Self::Etco), // Event timing codes
      (Version::ID3v22, "EQU") => decoder.decode_v2().map(Self::Equa), // Equalization
      (Version::ID3v22, "GEO") => decoder.decode_v2().map(Self::Geob), // General encapsulated object
      (Version::ID3v22, "IPL") => decoder.decode_v2().map(Self::Ipls), // Involved people list
      (Version::ID3v22, "LNK") => decoder.decode_v2().map(Self::Link), // Linked information
      (Version::ID3v22, "MCI") => decoder.decode_v2().map(Self::Mcdi), // Music CD Identifier
      (Version::ID3v22, "MLL") => decoder.decode_v2().map(Self::Mllt), // MPEG location lookup table
      (Version::ID3v22, "PIC") => decoder.decode_v2().map(Self::Apic), // Attached picture
      (Version::ID3v22, "POP") => decoder.decode_v2().map(Self::Popm), // Popularimeter
      (Version::ID3v22, "REV") => decoder.decode_v2().map(Self::Rvrb), // Reverb
      (Version::ID3v22, "RVA") => decoder.decode_v2().map(Self::Rvad), // Relative volume adjustment
      (Version::ID3v22, "SLT") => decoder.decode_v2().map(Self::Sylt), // Synchronized lyric/text
      (Version::ID3v22, "STC") => decoder.decode_v2().map(Self::Sytc), // Synced tempo codes
      (Version::ID3v22, "TAL") => decoder.decode_v2().map(Self::Text), // Album/Movie/Show title
      (Version::ID3v22, "TBP") => decoder.decode_v2().map(Self::Text), // BPM (Beats Per Minute)
      (Version::ID3v22, "TCM") => decoder.decode_v2().map(Self::Text), // Composer
      (Version::ID3v22, "TCO") => decoder.decode_v2().map(Self::Text), // Self type
      (Version::ID3v22, "TCR") => decoder.decode_v2().map(Self::Text), // Copyright message
      (Version::ID3v22, "TDA") => decoder.decode_v2().map(Self::Text), // Date
      (Version::ID3v22, "TDY") => decoder.decode_v2().map(Self::Text), // Playlist delay
      (Version::ID3v22, "TEN") => decoder.decode_v2().map(Self::Text), // Encoded by
      (Version::ID3v22, "TFT") => decoder.decode_v2().map(Self::Text), // File type
      (Version::ID3v22, "TIM") => decoder.decode_v2().map(Self::Text), // Time
      (Version::ID3v22, "TKE") => decoder.decode_v2().map(Self::Text), // Initial key
      (Version::ID3v22, "TLA") => decoder.decode_v2().map(Self::Text), // Language(s)
      (Version::ID3v22, "TLE") => decoder.decode_v2().map(Self::Text), // Length
      (Version::ID3v22, "TMT") => decoder.decode_v2().map(Self::Text), // Media type
      (Version::ID3v22, "TOA") => decoder.decode_v2().map(Self::Text), // Original artist(s)/performer(s)
      (Version::ID3v22, "TOF") => decoder.decode_v2().map(Self::Text), // Original filename
      (Version::ID3v22, "TOL") => decoder.decode_v2().map(Self::Text), // Original Lyricist(s)/text writer(s)
      (Version::ID3v22, "TOR") => decoder.decode_v2().map(Self::Text), // Original release year
      (Version::ID3v22, "TOT") => decoder.decode_v2().map(Self::Text), // Original album/Movie/Show title
      (Version::ID3v22, "TP1") => decoder.decode_v2().map(Self::Text), // Lead artist(s)/Lead performer(s)/Soloist(s)/Performing group
      (Version::ID3v22, "TP2") => decoder.decode_v2().map(Self::Text), // Band/Orchestra/Accompaniment
      (Version::ID3v22, "TP3") => decoder.decode_v2().map(Self::Text), // Conductor/Performer refinement
      (Version::ID3v22, "TP4") => decoder.decode_v2().map(Self::Text), // Interpreted, remixed, or otherwise modified by
      (Version::ID3v22, "TPA") => decoder.decode_v2().map(Self::Text), // Part of a set
      (Version::ID3v22, "TPB") => decoder.decode_v2().map(Self::Text), // Publisher
      (Version::ID3v22, "TRC") => decoder.decode_v2().map(Self::Text), // ISRC (International Standard Recording Code)
      (Version::ID3v22, "TRD") => decoder.decode_v2().map(Self::Text), // Recording dates
      (Version::ID3v22, "TRK") => decoder.decode_v2().map(Self::Text), // Track number/Position in set
      (Version::ID3v22, "TSI") => decoder.decode_v2().map(Self::Text), // Size
      (Version::ID3v22, "TSS") => decoder.decode_v2().map(Self::Text), // Software/hardware and settings used for encoding
      (Version::ID3v22, "TT1") => decoder.decode_v2().map(Self::Text), // Self group description
      (Version::ID3v22, "TT2") => decoder.decode_v2().map(Self::Text), // Title/Songname/Self description
      (Version::ID3v22, "TT3") => decoder.decode_v2().map(Self::Text), // Subtitle/Description refinement
      (Version::ID3v22, "TXT") => decoder.decode_v2().map(Self::Text), // Lyricist/text writer
      (Version::ID3v22, "TXX") => decoder.decode_v2().map(Self::Txxx), // User defined text information frame
      (Version::ID3v22, "TYE") => decoder.decode_v2().map(Self::Text), // Year
      (Version::ID3v22, "UFI") => decoder.decode_v2().map(Self::Ufid), // Unique file identifier
      (Version::ID3v22, "ULT") => decoder.decode_v2().map(Self::Uslt), // Unsychronized lyric/text transcription
      (Version::ID3v22, "WAF") => decoder.decode_v2().map(Self::Woaf), // Official audio file webpage
      (Version::ID3v22, "WAR") => decoder.decode_v2().map(Self::Woar), // Official artist/performer webpage
      (Version::ID3v22, "WAS") => decoder.decode_v2().map(Self::Woas), // Official audio source webpage
      (Version::ID3v22, "WCM") => decoder.decode_v2().map(Self::Wcom), // Commercial information
      (Version::ID3v22, "WCP") => decoder.decode_v2().map(Self::Wcop), // Copyright/Legal information
      (Version::ID3v22, "WPB") => decoder.decode_v2().map(Self::Wpub), // Publishers official webpage
      (Version::ID3v22, "WXX") => decoder.decode_v2().map(Self::Wxxx), // User defined URL link frame
      // =======================================================================
      // ID3v2.3 / ID3v2.4 Frames
      // =======================================================================
      (Version::ID3v23 | Version::ID3v24, "AENC") => decoder.decode().map(Self::Aenc), // [[#sec4.20|Audio encryption]]
      (Version::ID3v23 | Version::ID3v24, "APIC") => decoder.decode().map(Self::Apic), // [#sec4.15 Attached picture]
      (Version::ID3v23 | Version::ID3v24, "COMM") => decoder.decode().map(Self::Comm), // [#sec4.11 Comments]
      (Version::ID3v23 | Version::ID3v24, "COMR") => decoder.decode().map(Self::Comr), // [#sec4.25 Commercial frame]
      (Version::ID3v23 | Version::ID3v24, "ENCR") => decoder.decode().map(Self::Encr), // [#sec4.26 Encryption method registration]
      (Version::ID3v23, "EQUA") => decoder.decode().map(Self::Equa), // [#sec4.13 Equalization]
      (Version::ID3v23 | Version::ID3v24, "ETCO") => decoder.decode().map(Self::Etco), // [#sec4.6 Event timing codes]
      (Version::ID3v23 | Version::ID3v24, "GEOB") => decoder.decode().map(Self::Geob), // [#sec4.16 General encapsulated object]
      (Version::ID3v23 | Version::ID3v24, "GRID") => decoder.decode().map(Self::Grid), // [#sec4.27 Group identification registration]
      (Version::ID3v23, "IPLS") => decoder.decode().map(Self::Ipls), // [#sec4.4 Involved people list]
      (Version::ID3v23 | Version::ID3v24, "LINK") => decoder.decode().map(Self::Link), // [#sec4.21 Linked information]
      (Version::ID3v23 | Version::ID3v24, "MCDI") => decoder.decode().map(Self::Mcdi), // [#sec4.5 Music CD identifier]
      (Version::ID3v23 | Version::ID3v24, "MLLT") => decoder.decode().map(Self::Mllt), // [#sec4.7 MPEG location lookup table]
      (Version::ID3v23 | Version::ID3v24, "OWNE") => decoder.decode().map(Self::Owne), // [#sec4.24 Ownership frame]
      (Version::ID3v23 | Version::ID3v24, "PRIV") => decoder.decode().map(Self::Priv), // [#sec4.28 Private frame]
      (Version::ID3v23 | Version::ID3v24, "PCNT") => decoder.decode().map(Self::Pcnt), // [#sec4.17 Play counter]
      (Version::ID3v23 | Version::ID3v24, "POPM") => decoder.decode().map(Self::Popm), // [#sec4.18 Popularimeter]
      (Version::ID3v23 | Version::ID3v24, "POSS") => decoder.decode().map(Self::Poss), // [#sec4.22 Position synchronisation frame]
      (Version::ID3v23 | Version::ID3v24, "RBUF") => decoder.decode().map(Self::Rbuf), // [#sec4.19 Recommended buffer size]
      (Version::ID3v23, "RVAD") => decoder.decode().map(Self::Rvad), // [#sec4.12 Relative volume adjustment]
      (Version::ID3v23 | Version::ID3v24, "RVRB") => decoder.decode().map(Self::Rvrb), // [#sec4.14 Reverb]
      (Version::ID3v23 | Version::ID3v24, "SYLT") => decoder.decode().map(Self::Sylt), // [#sec4.10 Synchronized lyric/text]
      (Version::ID3v23 | Version::ID3v24, "SYTC") => decoder.decode().map(Self::Sytc), // [#sec4.8 Synchronized tempo codes]
      (Version::ID3v23 | Version::ID3v24, "TALB") => decoder.decode().map(Self::Text), // [#TALB Album/Movie/Show title]
      (Version::ID3v23 | Version::ID3v24, "TBPM") => decoder.decode().map(Self::Text), // [#TBPM BPM (beats per minute)]
      (Version::ID3v23 | Version::ID3v24, "TCOM") => decoder.decode().map(Self::Text), // [#TCOM Composer]
      (Version::ID3v23 | Version::ID3v24, "TCON") => decoder.decode().map(Self::Text), // [#TCON Self type]
      (Version::ID3v23 | Version::ID3v24, "TCOP") => decoder.decode().map(Self::Text), // [#TCOP Copyright message]
      (Version::ID3v23, "TDAT") => decoder.decode().map(Self::Text), // [#TDAT Date]
      (Version::ID3v23 | Version::ID3v24, "TDLY") => decoder.decode().map(Self::Text), // [#TDLY Playlist delay]
      (Version::ID3v23 | Version::ID3v24, "TENC") => decoder.decode().map(Self::Text), // [#TENC Encoded by]
      (Version::ID3v23 | Version::ID3v24, "TEXT") => decoder.decode().map(Self::Text), // [#TEXT Lyricist/Text writer]
      (Version::ID3v23 | Version::ID3v24, "TFLT") => decoder.decode().map(Self::Text), // [#TFLT File type]
      (Version::ID3v23, "TIME") => decoder.decode().map(Self::Text), // [#TIME Time]
      (Version::ID3v23 | Version::ID3v24, "TIT1") => decoder.decode().map(Self::Text), // [#TIT1 Self group description]
      (Version::ID3v23 | Version::ID3v24, "TIT2") => decoder.decode().map(Self::Text), // [#TIT2 Title/songname/content description]
      (Version::ID3v23 | Version::ID3v24, "TIT3") => decoder.decode().map(Self::Text), // [#TIT3 Subtitle/Description refinement]
      (Version::ID3v23 | Version::ID3v24, "TKEY") => decoder.decode().map(Self::Text), // [#TKEY Initial key]
      (Version::ID3v23 | Version::ID3v24, "TLAN") => decoder.decode().map(Self::Text), // [#TLAN Language(s)]
      (Version::ID3v23 | Version::ID3v24, "TLEN") => decoder.decode().map(Self::Text), // [#TLEN Length]
      (Version::ID3v23 | Version::ID3v24, "TMED") => decoder.decode().map(Self::Text), // [#TMED Media type]
      (Version::ID3v23 | Version::ID3v24, "TOAL") => decoder.decode().map(Self::Text), // [#TOAL Original album/movie/show title]
      (Version::ID3v23 | Version::ID3v24, "TOFN") => decoder.decode().map(Self::Text), // [#TOFN Original filename]
      (Version::ID3v23 | Version::ID3v24, "TOLY") => decoder.decode().map(Self::Text), // [#TOLY Original lyricist(s)/text writer(s)]
      (Version::ID3v23 | Version::ID3v24, "TOPE") => decoder.decode().map(Self::Text), // [#TOPE Original artist(s)/performer(s)]
      (Version::ID3v23, "TORY") => decoder.decode().map(Self::Text), // [#TORY Original release year]
      (Version::ID3v23 | Version::ID3v24, "TOWN") => decoder.decode().map(Self::Text), // [#TOWN File owner/licensee]
      (Version::ID3v23 | Version::ID3v24, "TPE1") => decoder.decode().map(Self::Text), // [#TPE1 Lead performer(s)/Soloist(s)]
      (Version::ID3v23 | Version::ID3v24, "TPE2") => decoder.decode().map(Self::Text), // [#TPE2 Band/orchestra/accompaniment]
      (Version::ID3v23 | Version::ID3v24, "TPE3") => decoder.decode().map(Self::Text), // [#TPE3 Conductor/performer refinement]
      (Version::ID3v23 | Version::ID3v24, "TPE4") => decoder.decode().map(Self::Text), // [#TPE4 Interpreted, remixed, or otherwise modified by]
      (Version::ID3v23 | Version::ID3v24, "TPOS") => decoder.decode().map(Self::Text), // [#TPOS Part of a set]
      (Version::ID3v23 | Version::ID3v24, "TPUB") => decoder.decode().map(Self::Text), // [#TPUB Publisher]
      (Version::ID3v23 | Version::ID3v24, "TRCK") => decoder.decode().map(Self::Text), // [#TRCK Track number/Position in set]
      (Version::ID3v23, "TRDA") => decoder.decode().map(Self::Text), // [#TRDA Recording dates]
      (Version::ID3v23 | Version::ID3v24, "TRSN") => decoder.decode().map(Self::Text), // [#TRSN Internet radio station name]
      (Version::ID3v23 | Version::ID3v24, "TRSO") => decoder.decode().map(Self::Text), // [#TRSO Internet radio station owner]
      (Version::ID3v23, "TSIZ") => decoder.decode().map(Self::Text), // [#TSIZ Size]
      (Version::ID3v23 | Version::ID3v24, "TSRC") => decoder.decode().map(Self::Text), // [#TSRC ISRC (international standard recording code)]
      (Version::ID3v23 | Version::ID3v24, "TSSE") => decoder.decode().map(Self::Text), // [#TSEE Software/Hardware and settings used for encoding]
      (Version::ID3v23, "TYER") => decoder.decode().map(Self::Text), // [#TYER Year]
      (Version::ID3v23 | Version::ID3v24, "TXXX") => decoder.decode().map(Self::Txxx), // [#TXXX User defined text information frame]
      (Version::ID3v23 | Version::ID3v24, "UFID") => decoder.decode().map(Self::Ufid), // [#sec4.1 Unique file identifier]
      (Version::ID3v23 | Version::ID3v24, "USER") => decoder.decode().map(Self::User), // [#sec4.23 Terms of use]
      (Version::ID3v23 | Version::ID3v24, "USLT") => decoder.decode().map(Self::Uslt), // [#sec4.9 Unsychronized lyric/text transcription]
      (Version::ID3v23 | Version::ID3v24, "WCOM") => decoder.decode().map(Self::Wcom), // [#WCOM Commercial information]
      (Version::ID3v23 | Version::ID3v24, "WCOP") => decoder.decode().map(Self::Wcop), // [#WCOP Copyright/Legal information]
      (Version::ID3v23 | Version::ID3v24, "WOAF") => decoder.decode().map(Self::Woaf), // [#WOAF Official audio file webpage]
      (Version::ID3v23 | Version::ID3v24, "WOAR") => decoder.decode().map(Self::Woar), // [#WOAR Official artist/performer webpage]
      (Version::ID3v23 | Version::ID3v24, "WOAS") => decoder.decode().map(Self::Woas), // [#WOAS Official audio source webpage]
      (Version::ID3v23 | Version::ID3v24, "WORS") => decoder.decode().map(Self::Wors), // [#WORS Official internet radio station homepage]
      (Version::ID3v23 | Version::ID3v24, "WPAY") => decoder.decode().map(Self::Wpay), // [#WPAY Payment]
      (Version::ID3v23 | Version::ID3v24, "WPUB") => decoder.decode().map(Self::Wpub), // [#WPUB Publishers official webpage]
      (Version::ID3v23 | Version::ID3v24, "WXXX") => decoder.decode().map(Self::Wxxx), // [#WXXX User defined URL link frame]
      // =======================================================================
      // ID3v2.4 Frames
      // =======================================================================
      (Version::ID3v24, "ASPI") => panic!("TODO: Decode ASPI"),
      (Version::ID3v24, "EQU2") => panic!("TODO: Decode EQU2"),
      (Version::ID3v24, "RVA2") => decoder.decode().map(Self::Rva2), // relative volume adjustment (2)
      (Version::ID3v24, "SEEK") => panic!("TODO: Decode SEEK"),
      (Version::ID3v24, "SIGN") => panic!("TODO: Decode SIGN"),
      (Version::ID3v24, "TDEN") => decoder.decode().map(Self::Text), // encoding time
      (Version::ID3v24, "TDOR") => decoder.decode().map(Self::Text), // original release time
      (Version::ID3v24, "TDRC") => decoder.decode().map(Self::Text), // recording time
      (Version::ID3v24, "TDRL") => decoder.decode().map(Self::Text), // release time
      (Version::ID3v24, "TDTG") => decoder.decode().map(Self::Text), // tagging time
      (Version::ID3v24, "TIPL") => decoder.decode().map(Self::Text), // involved people list
      (Version::ID3v24, "TMCL") => decoder.decode().map(Self::Text), // musician credits list
      (Version::ID3v24, "TMOO") => decoder.decode().map(Self::Text), // mood
      (Version::ID3v24, "TPRO") => decoder.decode().map(Self::Text), // produced notice
      (Version::ID3v24, "TSOA") => decoder.decode().map(Self::Text), // album sort order
      (Version::ID3v24, "TSOP") => decoder.decode().map(Self::Text), // performer sort order
      (Version::ID3v24, "TSOT") => decoder.decode().map(Self::Text), // title sort order
      (Version::ID3v24, "TSST") => decoder.decode().map(Self::Text), // set subtitle
      // =======================================================================
      // ID3v2 Chapter Frame Addendum v1.0
      // =======================================================================
      (Version::ID3v23 | Version::ID3v24, "CHAP") => decoder.decode().map(Self::Chap),
      (Version::ID3v23 | Version::ID3v24, "CTOC") => decoder.decode().map(Self::Ctoc),
      // =======================================================================
      // ID3v2 Accessibility Addendum v1.0
      // =======================================================================
      (Version::ID3v23 | Version::ID3v24, "ATXT") => decoder.decode().map(Self::Atxt),
      // =======================================================================
      // Unoffical Frames
      // =======================================================================
      (_, "RGAD") => panic!("TODO: Decode RGAD"),
      (_, "TCMP") => panic!("TODO: Decode TCMP"),
      (_, "TSO2") => panic!("TODO: Decode TSO2"),
      (_, "TSOC") => panic!("TODO: Decode TSOC"),
      (_, "XRVA") => panic!("TODO: Decode XRVA"),
      // =======================================================================
      // Unknown Frame
      // =======================================================================
      _ => panic!("Unknown Frame: {:?}", name),
    };

    assert!(decoder.is_empty());

    this
  }
}

impl Content<'static> {
  pub(crate) fn decode2(version: Version, name: &str, slice: &Slice, size: u32) -> Result<Self> {
    let bytes: Bytes = utils::decompress(slice, Some(size as usize))?;
    let slice: &Slice = bytes.as_slice();

    let content: Content<'_> = Content::decode(version, name, slice)?;

    Ok(content.into_owned())
  }
}

impl IntoOwned for Content<'_> {
  type Owned = Content<'static>;

  fn into_owned(self) -> Self::Owned {
    panic!("Content::into_owned");
  }
}
