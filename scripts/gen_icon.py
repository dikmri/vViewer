#!/usr/bin/env python3
"""Generate a minimal 512x512 PNG icon for vViewer (CI use)."""
import struct
import zlib
import os

def create_png(width: int, height: int, r: int, g: int, b: int) -> bytes:
    def chunk(tag: bytes, data: bytes) -> bytes:
        c = tag + data
        return struct.pack('>I', len(data)) + c + struct.pack('>I', zlib.crc32(c) & 0xFFFFFFFF)

    ihdr  = struct.pack('>IIBBBBB', width, height, 8, 2, 0, 0, 0)
    raw   = b''.join(b'\x00' + bytes([r, g, b]) * width for _ in range(height))
    idat  = zlib.compress(raw, 9)

    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', ihdr) + chunk(b'IDAT', idat) + chunk(b'IEND', b'')


if __name__ == '__main__':
    # Deep blue: #1e3a8a
    icon_bytes = create_png(512, 512, 30, 58, 138)
    out = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), 'app-icon.png')
    with open(out, 'wb') as f:
        f.write(icon_bytes)
    print(f'Created {out}')
