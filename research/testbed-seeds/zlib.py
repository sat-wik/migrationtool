#!/usr/bin/env python3
"""Print the hex of one hand-made zlib stream.

Research helper for the zlib seeds (BUILD-03). Run once by hand, never by the
tool or the tests. This file is named zlib.py to match the test-bed target id, so
it must be run in isolated mode (the script directory is then not on sys.path):

    python3 -I research/testbed-seeds/zlib.py --list
    python3 -I research/testbed-seeds/zlib.py stored_hello

Streams follow RFC 1950 (a 2-byte header, the deflate data, a big-endian
Adler-32 of the uncompressed data) and RFC 1951 (deflate blocks: a 3-bit block
header BFINAL and BTYPE, stored blocks with LEN and NLEN, fixed-Huffman blocks
section 3.2.6, dynamic-Huffman blocks section 3.2.7). Numeric fields are packed
least-significant bit first, Huffman codes most-significant bit first. Standard
library only, and no import of the zlib module (the Adler-32 is computed here),
so every byte is derived from the two RFCs.
"""
import struct
import sys


def adler32(data):
    a, b = 1, 0
    for byte in data:
        a = (a + byte) % 65521
        b = (b + a) % 65521
    return (b << 16) | a


class Bits:
    """Deflate bit packing: bits are filled from the least significant end."""

    def __init__(self):
        self.value = 0
        self.count = 0

    def num(self, value, width):
        """A numeric field: least-significant bit first."""
        for i in range(width):
            self.value |= ((value >> i) & 1) << self.count
            self.count += 1

    def code(self, value, width):
        """A Huffman code: most-significant bit first."""
        for i in range(width - 1, -1, -1):
            self.value |= ((value >> i) & 1) << self.count
            self.count += 1

    def to_bytes(self):
        return self.value.to_bytes((self.count + 7) // 8, "little")


def header(cmf=0x78, flevel=0, fdict=0):
    """CMF and FLG with FCHECK chosen so that the pair is a multiple of 31."""
    flg = (flevel << 6) | (fdict << 5)
    flg += (31 - ((cmf << 8) + flg) % 31) % 31
    return bytes([cmf, flg])


def stored(data, final=True):
    return bytes([1 if final else 0]) + struct.pack("<HH", len(data), len(data) ^ 0xFFFF) + data


def trailer(data):
    return struct.pack(">I", adler32(data))


def fixed_literal(bits, value):
    if value < 144:
        bits.code(0x30 + value, 8)
    elif value < 256:
        bits.code(0x190 + value - 144, 9)
    elif value < 280:
        bits.code(value - 256, 7)
    else:
        bits.code(0xC0 + value - 280, 8)


def fixed_block(items, final=True):
    """items: ("lit", byte) or ("match", length 3..10, distance 1..4)."""
    bits = Bits()
    bits.num(1 if final else 0, 1)
    bits.num(1, 2)  # BTYPE 01: fixed Huffman codes
    for item in items:
        if item[0] == "lit":
            fixed_literal(bits, item[1])
        else:
            _, length, distance = item
            assert 3 <= length <= 10 and 1 <= distance <= 4
            fixed_literal(bits, 257 + length - 3)  # length codes 257..264, no extra bits
            bits.code(distance - 1, 5)  # distance codes 0..3, no extra bits
    fixed_literal(bits, 256)  # end of block
    return bits.to_bytes()


def dynamic_block_aaaa(count):
    """One dynamic block of `count` times 'a': literal 97 and end-of-block get
    1-bit codes, there is one 1-bit distance code, and the code lengths are sent
    with a two-symbol code-length alphabet (symbol 1, symbol 18)."""
    bits = Bits()
    bits.num(1, 1)
    bits.num(2, 2)  # BTYPE 10: dynamic Huffman codes
    bits.num(0, 5)  # HLIT: 257 literal/length codes
    bits.num(0, 5)  # HDIST: 1 distance code
    bits.num(15, 4)  # HCLEN: 19 code-length codes
    order = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15]
    cl_len = {18: 1, 1: 1}
    for symbol in order:
        bits.num(cl_len.get(symbol, 0), 3)
    # Canonical codes: symbol 1 -> 0, symbol 18 -> 1 (equal lengths: lower symbol first).
    one, zeros = (0, 1), (1, 1)
    bits.code(*zeros); bits.num(97 - 11, 7)    # symbols 0..96: 97 zero lengths
    bits.code(*one)                            # symbol 97: length 1
    bits.code(*zeros); bits.num(138 - 11, 7)   # symbols 98..235
    bits.code(*zeros); bits.num(20 - 11, 7)    # symbols 236..255
    bits.code(*one)                            # symbol 256 (end of block): length 1
    bits.code(*one)                            # the distance code: length 1
    for _ in range(count):
        bits.code(0, 1)                        # literal 97
    bits.code(1, 1)                            # end of block
    return bits.to_bytes()


def cases():
    hello = b"hello"
    abc = b"abcabcabc"
    a40 = b"a" * 40
    good_stored = header() + stored(hello) + trailer(hello)
    return {
        "stored_hello": good_stored,
        "stored_empty": header() + stored(b"") + trailer(b""),
        "stored_two_blocks": header() + stored(b"he", final=False) + stored(b"llo") + trailer(hello),
        "fixed_huffman_literal": header() + fixed_block([("lit", 0x61)]) + trailer(b"a"),
        "fixed_huffman_with_match": header()
        + fixed_block([("lit", 0x61), ("lit", 0x62), ("lit", 0x63), ("match", 6, 3)])
        + trailer(abc),
        "fixed_huffman_high_literal": header() + fixed_block([("lit", 0xE9)]) + trailer(b"\xe9"),
        "dynamic_huffman_40_a": header() + dynamic_block_aaaa(40) + trailer(a40),
        "fixed_huffman_distance_too_far": header()
        + fixed_block([("lit", 0x61), ("match", 4, 3)])
        + trailer(b"aaaaa"),
        "preset_dictionary_flag": header(fdict=1) + struct.pack(">I", adler32(b"hello")) + stored(hello) + trailer(hello),
        "bad_adler32": header() + stored(hello) + struct.pack(">I", adler32(hello) ^ 1),
        "bad_header_check": bytes([0x78, 0x02]) + stored(hello) + trailer(hello),
        "bad_compression_method": header(cmf=0x79) + stored(hello) + trailer(hello),
        "window_too_large": header(cmf=0x88) + stored(hello) + trailer(hello),
        "reserved_block_type": header() + bytes([0x07]) + trailer(b""),
        "stored_len_nlen_mismatch": header() + bytes([1]) + struct.pack("<HH", 5, 5) + hello + trailer(hello),
        "truncated_after_header": good_stored[:2],
        "truncated_mid_stored": good_stored[:6],
        "missing_trailer": good_stored[:-4],
    }


def main(argv):
    table = cases()
    if len(argv) != 2 or (argv[1] != "--list" and argv[1] not in table):
        sys.stderr.write("usage: zlib.py --list | <case>\n")
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
