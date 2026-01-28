"""Intel HEX format utilities for generating test files."""

from dataclasses import dataclass
from typing import Iterator


@dataclass
class Segment:
    """A contiguous block of data at a specific address."""

    start_address: int
    data: bytes

    @property
    def end_address(self) -> int:
        return self.start_address + len(self.data) - 1

    def __len__(self) -> int:
        return len(self.data)


def checksum(data: bytes) -> int:
    """Calculate Intel HEX checksum (two's complement of sum)."""
    return (~sum(data) + 1) & 0xFF


def format_record(record_type: int, address: int, data: bytes = b"") -> str:
    """Format a single Intel HEX record.

    Record types:
        00 - Data
        01 - EOF
        02 - Extended Segment Address (shifts address by 16)
        04 - Extended Linear Address (upper 16 bits)
    """
    length = len(data)
    addr_hi = (address >> 8) & 0xFF
    addr_lo = address & 0xFF

    record_bytes = bytes([length, addr_hi, addr_lo, record_type]) + data
    cs = checksum(record_bytes)

    hex_data = data.hex().upper()
    return f":{length:02X}{address:04X}{record_type:02X}{hex_data}{cs:02X}"


def format_data_records(
    address: int, data: bytes, bytes_per_line: int = 16
) -> Iterator[str]:
    """Generate data records for a block of data, handling extended addressing.

    Uses HexView auto-mode rules:
    - 16 bits or less: no extended records
    - 17-20 bits: Extended Segment Address (type 02)
    - 21-32 bits: Extended Linear Address (type 04)
    """
    # Track current extended state: (record_type, value)
    # record_type: None (no extended yet), 0x02, or 0x04
    current_ext_type: int | None = None
    current_ext_value: int = 0
    offset = 0

    while offset < len(data):
        abs_addr = address + offset

        # Determine which extended address mode is needed
        if abs_addr <= 0xFFFF:
            # 16-bit address: no extended record needed (unless we previously
            # emitted one for higher addresses and wrapped back)
            needed_ext_type = None
            needed_ext_value = 0
        elif abs_addr <= 0xFFFFF:
            # 17-20 bit address: use Extended Segment Address (type 02)
            # Segment base = upper 4 bits of 20-bit address, shifted left
            # Formula: segment_base << 4 = upper 16 bits of effective address
            # For addr 0x10000-0x1FFFF: segment_base = 0x1000
            # For addr 0x20000-0x2FFFF: segment_base = 0x2000
            needed_ext_type = 0x02
            needed_ext_value = ((abs_addr >> 4) & 0xF000)
        else:
            # 21-32 bit address: use Extended Linear Address (type 04)
            needed_ext_type = 0x04
            needed_ext_value = (abs_addr >> 16) & 0xFFFF

        # Emit extended record if needed
        if needed_ext_type is not None and (
            current_ext_type != needed_ext_type
            or current_ext_value != needed_ext_value
        ):
            current_ext_type = needed_ext_type
            current_ext_value = needed_ext_value
            ext_data = bytes([(needed_ext_value >> 8) & 0xFF, needed_ext_value & 0xFF])
            yield format_record(needed_ext_type, 0x0000, ext_data)

        # Data record uses lower 16 bits of address
        record_addr = abs_addr & 0xFFFF

        # Don't cross 64K boundary within a single record
        remaining_in_bank = 0x10000 - record_addr
        chunk_size = min(bytes_per_line, len(data) - offset, remaining_in_bank)

        chunk = data[offset : offset + chunk_size]
        yield format_record(0x00, record_addr, chunk)

        offset += chunk_size


def format_eof() -> str:
    """Generate EOF record."""
    return format_record(0x01, 0x0000)


def segments_to_intel_hex(
    segments: list[Segment], bytes_per_line: int = 16
) -> str:
    """Convert segments to Intel HEX format string."""
    lines = []

    # Sort segments by address
    sorted_segments = sorted(segments, key=lambda s: s.start_address)

    for segment in sorted_segments:
        for record in format_data_records(
            segment.start_address, segment.data, bytes_per_line
        ):
            lines.append(record)

    lines.append(format_eof())
    return "\n".join(lines) + "\n"


def write_intel_hex(
    path: str, segments: list[Segment], bytes_per_line: int = 16
) -> None:
    """Write segments to Intel HEX file."""
    content = segments_to_intel_hex(segments, bytes_per_line)
    with open(path, "w") as f:
        f.write(content)
