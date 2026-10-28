//! The result frame a driver writes once after the call, and the request file it
//! reads (CONTEXT D-30).
//!
//! Both are little-endian. The frame is
//! `"MTRS"`, version, status, raw return value, bytes the C code wrote through
//! libc (discarded), slack check result, region count, then per region an id, a
//! length and the bytes, then `"MTRE"`. Later plans add regions without a new
//! version. Parsing is strict: magic, version, lengths and the end marker must
//! match and no byte may follow the frame.

/// Magic of a result frame.
pub const FRAME_MAGIC: [u8; 4] = *b"MTRS";
/// End marker of a result frame.
pub const FRAME_END: [u8; 4] = *b"MTRE";
/// Magic of a request file.
pub const REQUEST_MAGIC: [u8; 4] = *b"MTRQ";
/// Version of the frame and of the request layout.
pub const FRAME_VERSION: u32 = 1;

/// The status word of a frame whose setup step `k` returned something other than
/// its expectation is `SETUP_STATUS_BASE + k` (CONTEXT D-12); the raw return value
/// of the frame is then that step's return value, and the entry did not run. A
/// frame whose entry ran has status 0.
pub const SETUP_STATUS_BASE: u32 = 0x100;

/// Bytes before the first region: magic and six words.
const HEADER_LEN: usize = 28;

/// One region of the frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    /// The region id.
    pub id: u32,
    /// The region bytes.
    pub bytes: Vec<u8>,
}

/// A parsed result frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// Status word: 0 when the call ran, `SETUP_STATUS_BASE + k` when setup step
    /// `k` failed.
    pub status: u32,
    /// The raw return value of the call.
    pub ret: u32,
    /// Bytes the C code wrote through libc; the driver discarded them.
    pub discarded: u32,
    /// Slack check result: 0 unchanged, non-zero when a slack byte changed.
    pub slack: u32,
    /// The regions, in frame order.
    pub regions: Vec<Region>,
    /// Length of the whole frame in bytes.
    pub total_len: usize,
}

impl Frame {
    /// The region with `id`, if the frame has one.
    #[must_use]
    pub fn region(&self, id: u32) -> Option<&Region> {
        self.regions.iter().find(|region| region.id == id)
    }
}

/// Why bytes are not a result frame.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FrameError {
    /// The bytes end before the frame does.
    #[error("frame is truncated: {0}")]
    Truncated(&'static str),
    /// The first four bytes are not the frame magic.
    #[error("bad frame magic")]
    BadMagic,
    /// The version is not the one this parser reads.
    #[error("unsupported frame version {0}")]
    BadVersion(u32),
    /// The end marker is missing.
    #[error("bad frame end marker")]
    BadEnd,
    /// Bytes follow the end marker.
    #[error("{0} bytes follow the frame")]
    Trailing(usize),
}

fn word(bytes: &[u8], at: usize) -> Option<u32> {
    let slice = bytes.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

/// Parse `bytes` as one complete result frame.
///
/// # Errors
/// [`FrameError`] when the bytes are not exactly one well-formed frame.
pub fn parse_frame(bytes: &[u8]) -> Result<Frame, FrameError> {
    if bytes.len() < HEADER_LEN {
        return Err(FrameError::Truncated("header"));
    }
    if bytes[..4] != FRAME_MAGIC {
        return Err(FrameError::BadMagic);
    }
    let field = |at: usize| word(bytes, at).ok_or(FrameError::Truncated("header"));
    let version = field(4)?;
    if version != FRAME_VERSION {
        return Err(FrameError::BadVersion(version));
    }
    let status = field(8)?;
    let ret = field(12)?;
    let discarded = field(16)?;
    let slack = field(20)?;
    let count = field(24)?;
    let mut at = HEADER_LEN;
    let mut regions = Vec::new();
    for _ in 0..count {
        let id = word(bytes, at).ok_or(FrameError::Truncated("region id"))?;
        let len = word(bytes, at + 4).ok_or(FrameError::Truncated("region length"))?;
        let start = at + 8;
        let len = usize::try_from(len).map_err(|_| FrameError::Truncated("region bytes"))?;
        let end = start
            .checked_add(len)
            .ok_or(FrameError::Truncated("region bytes"))?;
        let body = bytes
            .get(start..end)
            .ok_or(FrameError::Truncated("region bytes"))?;
        regions.push(Region {
            id,
            bytes: body.to_vec(),
        });
        at = end;
    }
    let marker = bytes
        .get(at..at + 4)
        .ok_or(FrameError::Truncated("end marker"))?;
    if marker != FRAME_END {
        return Err(FrameError::BadEnd);
    }
    at += 4;
    if at != bytes.len() {
        return Err(FrameError::Trailing(bytes.len() - at));
    }
    Ok(Frame {
        status,
        ret,
        discarded,
        slack,
        regions,
        total_len: at,
    })
}

/// Encode a frame. The driver writes frames itself; this exists so tests and
/// later plans can build well-formed and malformed frames.
#[must_use]
pub fn encode_frame(frame: &Frame) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&FRAME_MAGIC);
    for value in [
        FRAME_VERSION,
        frame.status,
        frame.ret,
        frame.discarded,
        frame.slack,
    ] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    let count = u32::try_from(frame.regions.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&count.to_le_bytes());
    for region in &frame.regions {
        out.extend_from_slice(&region.id.to_le_bytes());
        let len = u32::try_from(region.bytes.len()).unwrap_or(u32::MAX);
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&region.bytes);
    }
    out.extend_from_slice(&FRAME_END);
    out
}
