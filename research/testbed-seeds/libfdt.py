#!/usr/bin/env python3
"""Print the hex of one hand-made flattened devicetree (FDT) blob.

Research helper for the libfdt seeds (BUILD-03). Run once by hand, never by the
tool or the tests:

    python3 -I research/testbed-seeds/libfdt.py --list
    python3 -I research/testbed-seeds/libfdt.py minimal_valid

Layout follows the Devicetree Specification, chapter 5 "Flattened Devicetree
(DTB) Format": a 40-byte big-endian header (version 17), the memory reservation
block, the structure block (FDT_BEGIN_NODE 1, FDT_END_NODE 2, FDT_PROP 3,
FDT_NOP 4, FDT_END 9) and the strings block. Standard library only.
"""
import struct
import sys

MAGIC = 0xD00DFEED
BEGIN_NODE, END_NODE, PROP, NOP, END = 1, 2, 3, 4, 9


def pad4(data):
    return data + b"\0" * (-len(data) % 4)


def u32(*values):
    return b"".join(struct.pack(">I", v) for v in values)


def node(name):
    return u32(BEGIN_NODE) + pad4(name.encode() + b"\0")


def prop(nameoff, value):
    return u32(PROP, len(value), nameoff) + pad4(value)


def blob(structure, strings, rsv=b"\0" * 16, **override):
    """Assemble a version-17 blob; keyword arguments replace header fields."""
    off_rsv = 0x28
    off_struct = off_rsv + len(rsv)
    off_strings = off_struct + len(structure)
    fields = {
        "magic": MAGIC,
        "totalsize": off_strings + len(strings),
        "off_dt_struct": off_struct,
        "off_dt_strings": off_strings,
        "off_mem_rsvmap": off_rsv,
        "version": 17,
        "last_comp_version": 16,
        "boot_cpuid_phys": 0,
        "size_dt_strings": len(strings),
        "size_dt_struct": len(structure),
    }
    fields.update(override)
    header = u32(*fields.values())
    return header + rsv + structure + strings


STRINGS = b"model\0reg\0compatible\0"
MODEL, REG, COMPAT = 0, 6, 10
SIMPLE = (
    node("")
    + prop(MODEL, b"mt\0")
    + u32(END_NODE)
    + u32(END)
)
NESTED = (
    node("")
    + prop(COMPAT, b"vendor,board\0")
    + node("cpus")
    + node("cpu@0")
    + prop(REG, u32(0))
    + u32(END_NODE, END_NODE, END_NODE, END)
)


def cases():
    good = blob(SIMPLE, STRINGS)
    return {
        "minimal_valid": good,
        "nested_nodes_valid": blob(NESTED, STRINGS),
        "memory_reservation_valid": blob(
            SIMPLE, STRINGS, rsv=u32(0, 0x80000000, 0, 0x1000) + b"\0" * 16
        ),
        "nop_tokens_valid": blob(
            u32(NOP) + node("") + u32(NOP) + prop(MODEL, b"mt\0") + u32(END_NODE, NOP, END),
            STRINGS,
        ),
        "header_only_truncated": good[:40],
        "truncated_mid_structure": good[:0x48],
        "bad_magic": blob(SIMPLE, STRINGS, magic=0xD00DFEEE),
        "version_too_old": blob(SIMPLE, STRINGS, version=15, last_comp_version=15),
        "version_too_new": blob(SIMPLE, STRINGS, version=19, last_comp_version=18),
        "totalsize_beyond_buffer": blob(SIMPLE, STRINGS, totalsize=0x1000),
        "structure_offset_unaligned": blob(SIMPLE, STRINGS, off_dt_struct=0x39),
        "structure_offset_out_of_range": blob(SIMPLE, STRINGS, off_dt_struct=0x7FFFFFF0),
        "missing_fdt_end": blob(node("") + u32(END_NODE), STRINGS),
        "unbalanced_end_node": blob(node("") + u32(END_NODE, END_NODE, END), STRINGS),
        "property_name_offset_out_of_range": blob(
            node("") + prop(0x1000, b"x\0") + u32(END_NODE, END), STRINGS
        ),
        "property_length_beyond_structure": blob(
            node("") + u32(PROP, 0x100, MODEL) + u32(END_NODE, END), STRINGS
        ),
    }


def main(argv):
    table = cases()
    if len(argv) != 2 or (argv[1] != "--list" and argv[1] not in table):
        sys.stderr.write("usage: libfdt.py --list | <case>\n")
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
