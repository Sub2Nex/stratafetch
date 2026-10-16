#!/usr/bin/env python3
"""Fail packaging if a PE lacks the activation context required by TaskDialogIndirect."""
import struct
import sys
import xml.etree.ElementTree as ET
from pathlib import Path


def verify(path):
    data = Path(path).read_bytes()
    if data[:2] != b"MZ":
        raise ValueError("Not a Windows executable")
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe:pe + 4] != b"PE\0\0":
        raise ValueError("Invalid PE header")
    sections = struct.unpack_from("<H", data, pe + 6)[0]
    optional_size = struct.unpack_from("<H", data, pe + 20)[0]
    optional = pe + 24
    magic = struct.unpack_from("<H", data, optional)[0]
    if magic not in (0x10B, 0x20B):
        raise ValueError("Unknown PE optional header")
    directories = optional + (112 if magic == 0x20B else 96)
    resource_rva, resource_size = struct.unpack_from("<II", data, directories + 16)
    if not resource_rva or not resource_size:
        raise ValueError("Missing embedded Windows application manifest (no resources)")

    def offset(rva, size=1):
        for index in range(sections):
            section = optional + optional_size + index * 40
            address, raw_size, raw_offset = struct.unpack_from("<III", data, section + 12)
            if address <= rva and rva - address + size <= raw_size:
                result = raw_offset + rva - address
                if result + size <= len(data):
                    return result
        raise ValueError("Unmapped PE resource data")

    root = offset(resource_rva, resource_size)

    def entries(relative):
        if relative + 16 > resource_size:
            raise ValueError("Invalid resource directory")
        named, ids = struct.unpack_from("<HH", data, root + relative + 12)
        if relative + 16 + (named + ids) * 8 > resource_size:
            raise ValueError("Truncated resource directory")
        return [struct.unpack_from("<II", data, root + relative + 16 + i * 8)
                for i in range(named + ids)]

    def directory(relative, resource_id):
        for name, child in entries(relative):
            if name == resource_id and child & 0x80000000:
                return child & 0x7FFFFFFF
        raise ValueError(f"Missing manifest resource directory {resource_id}")

    languages = entries(directory(directory(0, 24), 1))
    if not languages:
        raise ValueError("Empty application manifest resource")
    for _, entry in languages:
        if entry & 0x80000000 or entry + 16 > resource_size:
            raise ValueError("Invalid manifest language resource")
        rva, size = struct.unpack_from("<II", data, root + entry)
        start = offset(rva, size)
        xml = ET.fromstring(data[start:start + size])
        definition = xml.find("{urn:schemas-microsoft-com:asm.v1}assemblyIdentity")
        if definition is None or definition.get("processorArchitecture") == "*":
            raise ValueError("Invalid application definition identity")
        identities = [node for node in xml.iter() if node.tag.endswith("}assemblyIdentity")]
        if not any(node.get("name") == "Microsoft.Windows.Common-Controls"
                   and node.get("version") == "6.0.0.0"
                   and node.get("publicKeyToken") == "6595b64144ccf1df"
                   for node in identities):
            raise ValueError("Common Controls v6 dependency missing from embedded manifest")
    print(f"Verified Common Controls v6 application manifest: {path}")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit("Usage: verify-windows-manifest.py EXECUTABLE [EXECUTABLE ...]")
    try:
        for executable in sys.argv[1:]:
            verify(executable)
    except (ValueError, struct.error, ET.ParseError) as error:
        sys.exit(str(error))
