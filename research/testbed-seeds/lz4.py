#!/usr/bin/env python3
"""Print the hex of one hand-made LZ4 block or frame.

Research helper for the lz4 seeds (BUILD-03). Run once by hand, never by the
tool or the tests:

    python3 -I research/testbed-seeds/lz4.py --list
    python3 -I research/testbed-seeds/lz4.py literal_only_block

Blocks follow the LZ4 Block Format specification (a sequence is a token, extra
literal-length bytes, the literals, a 2-byte little-endian offset, extra
match-length bytes; the last sequence has literals only). Frames follow the LZ4
Frame Format specification: magic 0x184D2204, a frame descriptor (FLG, BD) and
a header checksum byte that is the second byte of XXH32 over the descriptor,
then blocks (4-byte size, bit 31 set for an uncompressed block) and an end mark.
Standard library only; XXH32 is implemented here (it is checked against the
published vectors XXH32("")=0x02CC5D05 and XXH32("abc")=0x32D153FF).
"""
import struct
import sys

P1, P2, P3, P4, P5 = 2654435761, 2246822519, 3266489917, 668265263, 374761393
M32 = 0xFFFFFFFF


def rotl(x, r):
    return ((x << r) | (x >> (32 - r))) & M32


def xxh32(data, seed=0):
    """XXH32 of `data` (inputs here are always shorter than 16 bytes)."""
    assert len(data) < 16
    h = (seed + P5 + len(data)) & M32
    i = 0
    while i + 4 <= len(data):
        h = (h + struct.unpack_from("<I", data, i)[0] * P3) & M32
        h = (rotl(h, 17) * P4) & M32
        i += 4
    while i < len(data):
        h = (h + data[i] * P5) & M32
        h = (rotl(h, 11) * P1) & M32
        i += 1
    h ^= h >> 15
    h = (h * P2) & M32
    h ^= h >> 13
    h = (h * P3) & M32
    h ^= h >> 16
    return h


assert xxh32(b"") == 0x02CC5D05 and xxh32(b"abc") == 0x32D153FF


def frame_header(flg, bd, extra=b""):
    descriptor = bytes([flg, bd]) + extra
    hc = (xxh32(descriptor) >> 8) & 0xFF
    return struct.pack("<I", 0x184D2204) + descriptor + bytes([hc])


def raw_block(data):
    return struct.pack("<I", 0x80000000 | len(data)) + data


END_MARK = b"\0\0\0\0"
LAST = bytes([0x50]) + b"efghi"  # a final sequence: five literals and no match


def cases():
    return {
        # blocks
        "literal_only_block": bytes([0x50]) + b"hello",
        "empty_block_single_zero_token": b"\x00",
        "match_with_offset_block": bytes([0x44]) + b"abcd" + b"\x04\x00" + LAST,
        "overlapping_match_block": bytes([0x14]) + b"a" + b"\x01\x00" + LAST,
        "long_literal_length_block": bytes([0xF0, 0x03]) + bytes(range(0x41, 0x41 + 18)),
        "long_match_length_block": bytes([0x4F]) + b"abcd" + b"\x04\x00" + b"\xff\x02" + LAST,
        "offset_zero_block": bytes([0x14]) + b"a" + b"\x00\x00" + LAST,
        "offset_beyond_output_block": bytes([0x24]) + b"ab" + b"\x05\x00" + LAST,
        "truncated_in_literals_block": bytes([0x44]) + b"abc",
        "truncated_offset_block": bytes([0x44]) + b"abcd" + b"\x04",
        "ends_after_match_block": bytes([0x44]) + b"abcd" + b"\x04\x00",
        "literal_length_overruns_block": bytes([0xF0, 0xFF, 0xFF, 0xFF, 0x01]),
        # frames
        "frame_uncompressed_block": frame_header(0x60, 0x40) + raw_block(b"hello") + END_MARK,
        "frame_with_content_size": frame_header(0x68, 0x40, struct.pack("<Q", 5))
        + raw_block(b"hello")
        + END_MARK,
        "frame_content_checksum_wrong": frame_header(0x64, 0x40) + raw_block(b"hello") + END_MARK
        + struct.pack("<I", 0),
        "frame_bad_magic": b"\x05\x22\x4d\x18" + frame_header(0x60, 0x40)[4:],
        "frame_bad_header_checksum": frame_header(0x60, 0x40)[:-1]
        + bytes([(frame_header(0x60, 0x40)[-1] + 1) & 0xFF]),
        "frame_bad_version": frame_header(0xA0, 0x40),
        "frame_reserved_block_size": frame_header(0x60, 0x00),
        "frame_truncated_descriptor": struct.pack("<I", 0x184D2204) + b"\x60",
    }


def main(argv):
    table = cases()
    if len(argv) != 2 or (argv[1] != "--list" and argv[1] not in table):
        sys.stderr.write("usage: lz4.py --list | <case>\n")
        return 2
    if argv[1] == "--list":
        print("\n".join(table))
        return 0
    data = table[argv[1]]
    for i in range(0, len(data), 16):
        print(" ".join("%02x" % b for b in data[i : i + 16]))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
